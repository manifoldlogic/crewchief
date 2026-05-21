//! Integration tests — end-to-end validation of the gunmetal system.
//!
//! These tests exercise multiple modules working together to validate
//! that the complete GUN protocol implementation is correct.

use std::sync::{Arc, Mutex};

use gunmetal::cert::{Certificate, CertWhat, CertWho};
use gunmetal::instance::{Gun, GunOptions};
use gunmetal::runtime::{sleep_async, spawn_async};
use gunmetal::sea;
use gunmetal::storage::{
    AsyncMemoryStorage, AsyncStorageAdapter, MemoryStorage, StorageAdapter, StorageEngine,
};
use gunmetal::sync::sync_pair;
use gunmetal::transport::ws_native::{WsNativeConfig, WsNativeTransport};
use gunmetal::types::{GunValue, Node};
use gunmetal::user::{AuthResult, CreateResult, User};
use gunmetal::wire;

fn new_gun() -> Gun {
    Gun::new(GunOptions::default())
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 1: Basic CRUD lifecycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn crud_lifecycle() {
    let gun = new_gun();

    // Create
    gun.get("profile").put(vec![
        ("name".into(), GunValue::Text("Alice".into())),
        ("age".into(), GunValue::Number(30.0)),
        ("active".into(), GunValue::Bool(true)),
    ]);

    // Read
    assert_eq!(
        gun.get("profile").get("name").val(),
        Some(GunValue::Text("Alice".into()))
    );
    assert_eq!(
        gun.get("profile").get("age").val(),
        Some(GunValue::Number(30.0))
    );

    // Update (partial merge)
    gun.get("profile")
        .put_kv("name", GunValue::Text("Alice Smith".into()));
    assert_eq!(
        gun.get("profile").get("name").val(),
        Some(GunValue::Text("Alice Smith".into()))
    );
    // Age should still be there
    assert_eq!(
        gun.get("profile").get("age").val(),
        Some(GunValue::Number(30.0))
    );

    // Delete (tombstone)
    gun.get("profile").put_kv("active", GunValue::Null);
    assert_eq!(
        gun.get("profile").get("active").val(),
        Some(GunValue::Null)
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 2: Graph with circular references
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn circular_graph_references() {
    let gun = new_gun();

    // Mark's boss is Fluffy, Fluffy's slave is Mark
    gun.get("mark")
        .put_kv("name", GunValue::Text("Mark".into()));
    gun.get("mark")
        .put_kv("boss", GunValue::Link("fluffy".into()));

    gun.get("fluffy")
        .put_kv("name", GunValue::Text("Fluffy".into()));
    gun.get("fluffy")
        .put_kv("species", GunValue::Text("kitty".into()));
    gun.get("fluffy")
        .put_kv("slave", GunValue::Link("mark".into()));

    // Traverse forward: mark → boss → name
    let boss_name = gun.get("mark").get("boss").get("name").val();
    assert_eq!(boss_name, Some(GunValue::Text("Fluffy".into())));

    // Traverse circular: mark → boss → slave → name
    let slave_name = gun.get("mark").get("boss").get("slave").get("name").val();
    assert_eq!(slave_name, Some(GunValue::Text("Mark".into())));

    // Traverse back: fluffy → slave → boss → species
    let species = gun
        .get("fluffy")
        .get("slave")
        .get("boss")
        .get("species")
        .val();
    assert_eq!(species, Some(GunValue::Text("kitty".into())));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 3: Collections with .set()
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn collection_set_and_map() {
    let gun = new_gun();

    // Create items
    gun.get("alice")
        .put_kv("name", GunValue::Text("Alice".into()));
    gun.get("bob")
        .put_kv("name", GunValue::Text("Bob".into()));
    gun.get("charlie")
        .put_kv("name", GunValue::Text("Charlie".into()));

    // Add to collection
    gun.get("users").set(gun.get("alice"));
    gun.get("users").set(gun.get("bob"));
    gun.get("users").set(gun.get("charlie"));

    // Map over collection
    let collected = Arc::new(Mutex::new(Vec::new()));
    let c = collected.clone();
    gun.get("users").map(None, move |val, key| {
        c.lock().unwrap().push((key, val));
    });

    let items = collected.lock().unwrap();
    assert_eq!(items.len(), 3);
    assert!(items
        .iter()
        .any(|(_, v)| *v == GunValue::Link("alice".into())));
    assert!(items
        .iter()
        .any(|(_, v)| *v == GunValue::Link("bob".into())));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 4: Realtime subscriptions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn realtime_subscription_chain() {
    let gun = new_gun();

    // Seed initial data
    gun.get("counter")
        .put_kv("val", GunValue::Number(0.0));

    // Subscribe
    let history = Arc::new(Mutex::new(Vec::new()));
    let h = history.clone();
    let listener = gun.get("counter").get("val").on(move |val, _key| {
        h.lock().unwrap().push(val);
    });

    // Update multiple times
    gun.get("counter")
        .put_kv("val", GunValue::Number(1.0));
    gun.get("counter")
        .put_kv("val", GunValue::Number(2.0));
    gun.get("counter")
        .put_kv("val", GunValue::Number(3.0));

    let h = history.lock().unwrap();
    // Should have: initial(0) + 3 updates = 4 callbacks
    assert_eq!(h.len(), 4);
    assert_eq!(h[0], GunValue::Number(0.0));
    assert_eq!(h[3], GunValue::Number(3.0));

    // Unsubscribe
    drop(h);
    gun.get("counter").get("val").off(listener);

    gun.get("counter")
        .put_kv("val", GunValue::Number(4.0));

    let h = history.lock().unwrap();
    assert_eq!(h.len(), 4); // no new callbacks
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 5: Two peers sync with conflict resolution
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn peer_sync_with_ham_conflict_resolution() {
    let peer_a = new_gun();
    let peer_b = new_gun();

    let (mut sync_a, mut sync_b) = sync_pair(peer_a.clone(), peer_b.clone());

    // A writes
    peer_a.get("doc").put(vec![
        ("title".into(), GunValue::Text("Hello World".into())),
        ("author".into(), GunValue::Text("Alice".into())),
        ("version".into(), GunValue::Number(1.0)),
    ]);
    sync_a.flush();

    // Verify B received everything
    assert_eq!(
        peer_b.get("doc").get("title").val(),
        Some(GunValue::Text("Hello World".into()))
    );
    assert_eq!(
        peer_b.get("doc").get("author").val(),
        Some(GunValue::Text("Alice".into()))
    );

    // B writes different keys — no conflict
    peer_b
        .get("doc")
        .put_kv("reviewed", GunValue::Bool(true));
    sync_b.flush();

    // A should see B's addition
    assert_eq!(
        peer_a.get("doc").get("reviewed").val(),
        Some(GunValue::Bool(true))
    );

    // Both still have their original data
    assert_eq!(
        peer_a.get("doc").get("title").val(),
        Some(GunValue::Text("Hello World".into()))
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 6: Sync triggers remote listeners
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sync_triggers_remote_subscriptions() {
    let peer_a = new_gun();
    let peer_b = new_gun();

    let (mut sync_a, _sync_b) = sync_pair(peer_a.clone(), peer_b.clone());

    // B subscribes before A writes
    let messages = Arc::new(Mutex::new(Vec::new()));
    let m = messages.clone();
    peer_b.get("chat").get("latest").on(move |val, _key| {
        m.lock().unwrap().push(val);
    });

    // A sends a message
    peer_a
        .get("chat")
        .put_kv("latest", GunValue::Text("hello from A".into()));
    sync_a.flush();

    // B's listener should have fired
    let msgs = messages.lock().unwrap();
    assert!(msgs.contains(&GunValue::Text("hello from A".into())));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 7: Storage persist and reload
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn persist_and_reload_graph() {
    // Phase 1: Write data with persistence
    let gun1 = new_gun();
    let engine1 = StorageEngine::new(gun1.clone(), MemoryStorage::new());

    gun1.get("settings").put(vec![
        ("theme".into(), GunValue::Text("dark".into())),
        ("lang".into(), GunValue::Text("en".into())),
        ("fontSize".into(), GunValue::Number(14.0)),
    ]);

    gun1.get("user_data")
        .put_kv("name", GunValue::Text("TestUser".into()));
    gun1.get("user_data")
        .put_kv("prefs", GunValue::Link("settings".into()));

    // Extract storage contents
    let adapter = engine1.adapter();
    let store = adapter.lock().unwrap();
    let all_entries = store.scan("").unwrap();
    drop(store);

    // Phase 2: New instance, load from storage
    let gun2 = new_gun();
    let mut fresh_store = MemoryStorage::new();
    for (k, v) in all_entries {
        fresh_store.put(&k, &v).unwrap();
    }

    let engine2 = StorageEngine::new(gun2.clone(), fresh_store);
    engine2.load_all();

    // Verify all data restored
    assert_eq!(
        gun2.get("settings").get("theme").val(),
        Some(GunValue::Text("dark".into()))
    );
    assert_eq!(
        gun2.get("settings").get("lang").val(),
        Some(GunValue::Text("en".into()))
    );
    assert_eq!(
        gun2.get("settings").get("fontSize").val(),
        Some(GunValue::Number(14.0))
    );
    assert_eq!(
        gun2.get("user_data").get("name").val(),
        Some(GunValue::Text("TestUser".into()))
    );

    // Link should be preserved
    let prefs = gun2.get("user_data").get("prefs").val();
    assert_eq!(prefs, Some(GunValue::Link("settings".into())));

    // Following the link should work
    let theme = gun2.get("user_data").get("prefs").get("theme").val();
    assert_eq!(theme, Some(GunValue::Text("dark".into())));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 8: SEA sign/verify + encrypt/decrypt roundtrip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sea_full_crypto_roundtrip() {
    // Generate two key pairs
    let alice = sea::pair().unwrap();
    let bob = sea::pair().unwrap();

    // Alice signs a message
    let data = serde_json::json!({"message": "Hello Bob!", "timestamp": 1234567890});
    let signed = sea::sign(&data, &alice.priv_key, &alice.pub_key).unwrap();

    // Bob verifies Alice's signature
    let verified = sea::verify(&signed, &alice.pub_key).unwrap();
    assert_eq!(verified, data);

    // Verifying with wrong key fails
    assert!(sea::verify(&signed, &bob.pub_key).is_err());

    // Derive shared secret (ECDH)
    let secret_ab = sea::secret(&bob.epub, &alice.epriv).unwrap();
    let secret_ba = sea::secret(&alice.epub, &bob.epriv).unwrap();
    assert_eq!(secret_ab, secret_ba);

    // Alice encrypts for Bob using shared secret
    let secret_msg = serde_json::json!("Top secret: the cat is on the roof");
    let encrypted = sea::encrypt(&secret_msg, &secret_ab).unwrap();

    // Bob decrypts using the same shared secret
    let decrypted = sea::decrypt(&encrypted, &secret_ba).unwrap();
    assert_eq!(decrypted, secret_msg);

    // Eve can't decrypt (different key)
    let eve = sea::pair().unwrap();
    assert!(sea::decrypt(&encrypted, &eve.epriv).is_err());
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 9: PBKDF2 work for password hashing
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sea_work_password_hashing() {
    let password = "correct horse battery staple";
    let salt = "unique_salt_123";

    // Same input → same output (deterministic)
    let hash1 = sea::work(password, Some(salt)).unwrap();
    let hash2 = sea::work(password, Some(salt)).unwrap();
    assert_eq!(hash1, hash2);

    // Different password → different hash
    let hash3 = sea::work("wrong password", Some(salt)).unwrap();
    assert_ne!(hash1, hash3);

    // Hash is non-empty base64
    assert!(hash1.len() > 20);
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 10: User create → auth → write → leave → re-auth → read
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn user_full_lifecycle() {
    let gun = new_gun();
    let mut user = User::new(gun.clone());

    // Create account
    let pub_key = match user.create("fulltest_user", "secure_password_123") {
        CreateResult::Ok { pub_key } => pub_key,
        CreateResult::Err { err } => panic!("Create failed: {}", err),
    };

    // Should be auto-logged in
    let auth = user.is_authenticated().unwrap();
    assert_eq!(auth.alias, "fulltest_user");
    assert_eq!(auth.pub_key, pub_key);

    // Write to user namespace
    user.get("profile")
        .unwrap()
        .put_value(GunValue::Text("My Profile".into()));
    user.get("settings")
        .unwrap()
        .put_value(GunValue::Text("dark_mode".into()));

    // Verify data is stored under ~pubKey
    let user_soul = format!("~{}", pub_key);
    assert_eq!(
        gun.get(&user_soul).get("profile").val(),
        Some(GunValue::Text("My Profile".into()))
    );

    // Log out
    user.leave();
    assert!(user.is_authenticated().is_none());
    assert!(user.get("profile").is_none()); // can't read user data when logged out

    // Re-authenticate with password
    let mut user2 = User::new(gun.clone());
    match user2.auth_with_password("fulltest_user", "secure_password_123") {
        AuthResult::Ok(auth) => {
            assert_eq!(auth.pub_key, pub_key);
            assert_eq!(auth.alias, "fulltest_user");
            // Key pair should be restored
            assert!(!auth.pair.priv_key.is_empty());
            assert!(!auth.pair.epriv.is_empty());
        }
        AuthResult::Err { err } => panic!("Auth failed: {}", err),
    }

    // Data should still be accessible
    assert_eq!(
        gun.get(&user_soul).get("profile").val(),
        Some(GunValue::Text("My Profile".into()))
    );
    assert_eq!(
        gun.get(&user_soul).get("settings").val(),
        Some(GunValue::Text("dark_mode".into()))
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 11: Wire protocol interop
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn wire_protocol_interop() {
    let gun = new_gun();

    // Simulate receiving a PUT from an external GUN peer
    let mut node = Node::new("external_peer");
    node.put("status", GunValue::Text("online".into()), 50.0);
    node.put("version", GunValue::Text("0.2020".into()), 50.0);
    node.put(
        "relay",
        GunValue::Link("relay_server".into()),
        50.0,
    );

    let msg = wire::put_message("external_msg_1", &[&node]);
    let json = wire::serialize_message(&msg).unwrap();

    // Parse and apply
    let parsed = wire::parse_message(&json).unwrap();
    gun.receive(&parsed);

    // Data should be in the graph
    assert_eq!(
        gun.get("external_peer").get("status").val(),
        Some(GunValue::Text("online".into()))
    );
    assert_eq!(
        gun.get("external_peer").get("relay").val(),
        Some(GunValue::Link("relay_server".into()))
    );

    // Generate outgoing message
    gun.get("local_data")
        .put_kv("msg", GunValue::Text("hello peers".into()));

    let outgoing = gun.graph(|graph| {
        let node = graph.get_node("local_data").unwrap();
        let msg = wire::put_message("out_1", &[node]);
        wire::serialize_message(&msg).unwrap()
    });

    // Should be valid JSON that another GUN peer could parse
    let reparsed = wire::parse_message(&outgoing).unwrap();
    assert!(reparsed.put.is_some());
    let nodes = wire::json_to_graph(reparsed.put.as_ref().unwrap());
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].soul(), "local_data");
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 12: Sync + Storage combined
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sync_with_persistence() {
    let peer_a = new_gun();
    let peer_b = new_gun();

    // B has storage
    let _engine_b = StorageEngine::new(peer_b.clone(), MemoryStorage::new());

    let (mut sync_a, _sync_b) = sync_pair(peer_a.clone(), peer_b.clone());

    // A writes and syncs
    peer_a
        .get("shared_doc")
        .put_kv("content", GunValue::Text("Important data".into()));
    sync_a.flush();

    // B should have it in graph
    assert_eq!(
        peer_b.get("shared_doc").get("content").val(),
        Some(GunValue::Text("Important data".into()))
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 13: LEX queries with map
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn lex_filtered_map() {
    let gun = new_gun();

    // Time-series data
    gun.get("logs").put(vec![
        ("2024/01/01:09:00".into(), GunValue::Text("boot".into())),
        ("2024/01/01:09:15".into(), GunValue::Text("ready".into())),
        ("2024/01/02:10:00".into(), GunValue::Text("error".into())),
        ("2024/02/01:08:00".into(), GunValue::Text("update".into())),
    ]);

    // Query only January
    let january = Arc::new(Mutex::new(Vec::new()));
    let j = january.clone();
    let lex = gunmetal::lex::Lex::prefix("2024/01/");
    gun.get("logs").map(Some(&lex), move |val, key| {
        j.lock().unwrap().push((key, val));
    });

    let items = january.lock().unwrap();
    assert_eq!(items.len(), 3); // 3 entries in January
    assert!(items.iter().all(|(k, _)| k.starts_with("2024/01/")));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 14: Multiple clones share state
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn clone_shares_state() {
    let gun1 = new_gun();
    let gun2 = gun1.clone(); // cheap clone, shared state

    gun1.get("shared")
        .put_kv("x", GunValue::Number(42.0));

    assert_eq!(
        gun2.get("shared").get("x").val(),
        Some(GunValue::Number(42.0))
    );

    gun2.get("shared")
        .put_kv("y", GunValue::Number(99.0));

    assert_eq!(
        gun1.get("shared").get("y").val(),
        Some(GunValue::Number(99.0))
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 15: User with SEA — signed data
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn user_with_sea_signing() {
    let gun = new_gun();
    let mut user = User::new(gun.clone());

    user.create("signer", "password123456");
    let auth = user.is_authenticated().unwrap().clone();

    // Sign data with the user's key
    let data = serde_json::json!({"action": "transfer", "amount": 100});
    let signed = sea::sign(&data, &auth.pair.priv_key, &auth.pair.pub_key).unwrap();

    // Anyone can verify using the public key
    let verified = sea::verify(&signed, &auth.pair.pub_key).unwrap();
    assert_eq!(verified, data);

    // Store the signed data in the graph
    gun.get("transactions")
        .put_kv("tx1", GunValue::Text(signed.clone()));

    // Read back and verify
    let stored = gun.get("transactions").get("tx1").val().unwrap();
    if let GunValue::Text(signed_str) = stored {
        let verified2 = sea::verify(&signed_str, &auth.pair.pub_key).unwrap();
        assert_eq!(verified2, data);
    } else {
        panic!("Expected Text value");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Phase 2-4 Integration Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uuid_set_value_collection() {
    let gun = new_gun();

    // Add items to a collection using set_value (UUID-keyed)
    let k1 = gun.get("messages").set_value(GunValue::Text("hello".into()));
    let k2 = gun.get("messages").set_value(GunValue::Text("world".into()));
    let k3 = gun.get("messages").set_value(GunValue::Number(42.0));

    // All keys should be unique UUIDs
    assert_ne!(k1, k2);
    assert_ne!(k2, k3);
    assert_ne!(k1, k3);

    // All values should be retrievable
    assert_eq!(
        gun.get("messages").get(&k1).val(),
        Some(GunValue::Text("hello".into()))
    );
    assert_eq!(
        gun.get("messages").get(&k2).val(),
        Some(GunValue::Text("world".into()))
    );
    assert_eq!(
        gun.get("messages").get(&k3).val(),
        Some(GunValue::Number(42.0))
    );

    // Keys should be reasonable length (~20 chars)
    assert!(k1.len() >= 15 && k1.len() <= 30, "UUID length: {}", k1.len());
}

#[test]
fn eviction_with_subscriptions_auto_pin() {
    // Create a gun with very low eviction limits
    let gun = Gun::default_instance();

    // Subscribe to a node — should auto-pin it
    let received = Arc::new(Mutex::new(Vec::new()));
    let r = received.clone();
    let listener_id = gun.get("pinned_node").get("val").on(move |val, _key| {
        r.lock().unwrap().push(val);
    });

    // Write data to the pinned node
    gun.get("pinned_node")
        .put_kv("val", GunValue::Text("important".into()));

    // Verify the subscription fired
    assert!(received.lock().unwrap().len() >= 1);

    // The node should be pinned in the graph
    let is_pinned = gun.graph(|g| g.is_pinned("pinned_node"));
    assert!(is_pinned, "subscribed node should be auto-pinned");

    // Unsubscribe — should unpin
    gun.get("pinned_node").get("val").off(listener_id);

    let is_pinned_after = gun.graph(|g| g.is_pinned("pinned_node"));
    assert!(
        !is_pinned_after,
        "unsubscribed node should be unpinned"
    );
}

#[test]
fn get_request_across_sync_pair() {
    let gun_a = new_gun();
    let gun_b = new_gun();

    // B has data that A doesn't
    gun_b.get("remote").put(vec![
        ("name".into(), GunValue::Text("Remote Data".into())),
        ("version".into(), GunValue::Number(2.0)),
    ]);

    let (mut sync_a, _sync_b) = sync_pair(gun_a.clone(), gun_b.clone());

    // A requests data from B via GET
    sync_a.request("remote", None);

    // A should now have the data (GET response routed back)
    assert_eq!(
        gun_a.get("remote").get("name").val(),
        Some(GunValue::Text("Remote Data".into()))
    );
    assert_eq!(
        gun_a.get("remote").get("version").val(),
        Some(GunValue::Number(2.0))
    );
}

#[test]
fn certificate_delegated_write_flow() {
    // Alice creates a certificate granting Bob write access
    let alice_pair = sea::pair().unwrap();
    let bob_pair = sea::pair().unwrap();

    let cert = Certificate::create(
        CertWho::PubKey(bob_pair.pub_key.clone()),
        CertWhat::Prefix("shared/".to_string()),
        None, // no expiry
        &alice_pair.pub_key,
        &alice_pair.priv_key,
    )
    .unwrap();

    // Verify the certificate
    assert!(cert.verify().unwrap());

    // Certificate grants Bob access to shared/ paths
    assert!(cert.grants_access(&bob_pair.pub_key, "shared/doc1", 0.0));
    assert!(cert.grants_access(&bob_pair.pub_key, "shared/nested/deep", 0.0));

    // But not to other paths
    assert!(!cert.grants_access(&bob_pair.pub_key, "private/secret", 0.0));

    // Store certificate in the graph
    let gun = new_gun();
    let cert_id = cert.cert_id();
    let alice_soul = format!("~{}", alice_pair.pub_key);
    let cert_key = format!("certs/{}", cert_id);
    gun.get(&alice_soul)
        .put_kv(&cert_key, cert.to_gun_value());

    // Retrieve and verify from graph
    let stored = gun.get(&alice_soul).get(&cert_key).val().unwrap();
    let restored = Certificate::from_gun_value(&stored).unwrap();
    assert!(restored.verify().unwrap());
    assert!(restored.grants_access(&bob_pair.pub_key, "shared/doc1", 0.0));
}

#[test]
fn signed_chain_end_to_end() {
    let gun = new_gun();
    let mut user = User::new(gun.clone());

    // Create user and write signed data
    user.create("signer_e2e", "password12345");
    let pub_key = user.is_authenticated().unwrap().pub_key.clone();

    // Write through signed chain
    let profile = user.get_signed("bio").unwrap();
    profile.put_value(GunValue::Text("I am Alice".into()));

    // Read back through signed chain — should verify and return original value
    let val = profile.val();
    assert_eq!(val, Some(GunValue::Text("I am Alice".into())));

    // Raw value in graph should be SEA-signed
    let user_soul = format!("~{}", pub_key);
    let raw = gun.get(&user_soul).get("bio").val().unwrap();
    match raw {
        GunValue::Text(s) => assert!(s.starts_with("SEA{")),
        _ => panic!("expected signed text"),
    }

    // Another user verifying with the right public key should succeed
    let raw_text = match gun.get(&user_soul).get("bio").val().unwrap() {
        GunValue::Text(s) => s,
        _ => panic!("expected text"),
    };
    let verified = sea::verify_signed_value(&raw_text, &pub_key).unwrap();
    assert_eq!(verified, GunValue::Text("I am Alice".into()));
}

#[test]
fn sync_with_storage_and_eviction() {
    // Write data, persist to storage, then verify it survives
    let gun = new_gun();
    let engine = StorageEngine::new(gun.clone(), MemoryStorage::new());

    // Write multiple nodes
    gun.get("node_a").put_kv("x", GunValue::Number(1.0));
    gun.get("node_b").put_kv("y", GunValue::Number(2.0));
    gun.get("node_c").put_kv("z", GunValue::Number(3.0));

    // Verify storage has the data
    let adapter = engine.adapter();
    let store = adapter.lock().unwrap();
    assert!(store.get("node_a\x1Bx").unwrap().is_some());
    assert!(store.get("node_b\x1By").unwrap().is_some());
    assert!(store.get("node_c\x1Bz").unwrap().is_some());
    drop(store);

    // Load all from storage into a new gun instance
    let gun2 = new_gun();
    let entries = {
        let store = adapter.lock().unwrap();
        store.scan("").unwrap()
    };
    let mut fresh = MemoryStorage::new();
    for (k, v) in entries {
        fresh.put(&k, &v).unwrap();
    }
    let engine2 = StorageEngine::new(gun2.clone(), fresh);
    engine2.load_all();

    // All data should be restored
    assert_eq!(gun2.get("node_a").get("x").val(), Some(GunValue::Number(1.0)));
    assert_eq!(gun2.get("node_b").get("y").val(), Some(GunValue::Number(2.0)));
    assert_eq!(gun2.get("node_c").get("z").val(), Some(GunValue::Number(3.0)));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 22: WebSocket transport loopback
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn websocket_loopback_send_receive() {
    use tokio::net::TcpListener;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (server_tx, mut server_rx) = tokio::sync::mpsc::channel::<String>(10);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut sink, mut read) = ws.split();

        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(text) = msg {
                let _ = server_tx.send(text.to_string()).await;
                sink.send(Message::Text(format!("echo:{}", text).into()))
                    .await
                    .unwrap();
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = WsNativeTransport::new(WsNativeConfig::default());
    let url = format!("ws://127.0.0.1:{}", addr.port());
    let peer_id = client.connect(&url).await.unwrap();

    let test_msg = r##"{"#":"int-test","put":{"node":{"_":{"#":"node",">":{"k":1}},"k":"v"}}}"##;
    client.send(&peer_id, test_msg).await.unwrap();

    let received = server_rx.recv().await.unwrap();
    assert!(received.contains("int-test"));
    assert!(received.contains("node"));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 23: Async storage engine round-trip
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn async_storage_roundtrip() {
    use gunmetal::storage::StoredValue;

    let store = AsyncMemoryStorage::new();

    store
        .put(
            "soul\x1Bname".into(),
            StoredValue {
                value: GunValue::Text("Alice".into()),
                state: 100.0,
            },
        )
        .await
        .unwrap();

    store
        .put(
            "soul\x1Bage".into(),
            StoredValue {
                value: GunValue::Number(30.0),
                state: 101.0,
            },
        )
        .await
        .unwrap();

    store
        .put(
            "other\x1Bfoo".into(),
            StoredValue {
                value: GunValue::Bool(true),
                state: 102.0,
            },
        )
        .await
        .unwrap();

    let name = store.get("soul\x1Bname".into()).await.unwrap();
    assert_eq!(name.unwrap().value, GunValue::Text("Alice".into()));

    let missing = store.get("nonexistent".into()).await.unwrap();
    assert!(missing.is_none());

    let soul_entries = store.scan("soul\x1B".into()).await.unwrap();
    assert_eq!(soul_entries.len(), 2);

    store.delete("soul\x1Bage".into()).await.unwrap();
    let deleted = store.get("soul\x1Bage".into()).await.unwrap();
    assert!(deleted.is_none());

    assert_eq!(store.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 24: Certificate create + verify + revoke
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn certificate_create_verify_revoke_flow() {
    let alice = sea::pair().unwrap();
    let bob = sea::pair().unwrap();

    let cert = Certificate::create(
        CertWho::PubKey(bob.pub_key.clone()),
        CertWhat::Prefix("shared/".into()),
        None,
        &alice.pub_key,
        &alice.priv_key,
    )
    .unwrap();

    assert!(cert.verify().unwrap());

    let now = 0.0_f64; // no expiry on cert, so timestamp doesn't matter
    assert!(cert.grants_access(&bob.pub_key, "shared/data", now));
    assert!(cert.grants_access(&bob.pub_key, "shared/nested/deep", now));
    assert!(!cert.grants_access(&bob.pub_key, "private/secret", now));

    let eve = sea::pair().unwrap();
    assert!(!cert.grants_access(&eve.pub_key, "shared/data", now));

    let gun = new_gun();
    let cert_soul = format!("~{}/certs/{}", alice.pub_key, cert.cert_id());
    gun.get(&cert_soul).put_kv("cert", GunValue::Null);
    let revoked = gun.get(&cert_soul).get("cert").val();
    assert_eq!(revoked, Some(GunValue::Null));
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 25: Auto-signing end-to-end
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn auto_signing_end_to_end() {
    let gun = new_gun();
    let mut user = User::new(gun.clone());

    let result = user.create("signer", "password12345");
    assert!(matches!(result, CreateResult::Ok { .. }));

    let auth = user.auth_with_password("signer", "password12345");
    assert!(matches!(auth, AuthResult::Ok(_)));

    let signed_chain = user.get_signed("secret_data").unwrap();
    signed_chain.put_value(GunValue::Text("classified".into()));

    let read_back = user.get_signed("secret_data").unwrap();
    let val = read_back.val();
    assert_eq!(val, Some(GunValue::Text("classified".into())));

    let unsigned_chain = user.get("secret_data").unwrap();
    let raw = unsigned_chain.val();
    assert!(raw.is_some());
    if let Some(GunValue::Text(raw_text)) = raw {
        assert!(raw_text.starts_with("SEA{"), "value should be signed: {}", raw_text);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Scenario 26: Async runtime spawn + sleep
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn async_runtime_spawn_and_sleep() {
    use std::sync::atomic::{AtomicU32, Ordering};

    let counter = Arc::new(AtomicU32::new(0));

    let c1 = counter.clone();
    spawn_async(async move {
        c1.fetch_add(1, Ordering::SeqCst);
    });

    let c2 = counter.clone();
    spawn_async(async move {
        sleep_async(std::time::Duration::from_millis(10)).await;
        c2.fetch_add(10, Ordering::SeqCst);
    });

    sleep_async(std::time::Duration::from_millis(50)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 11);
}

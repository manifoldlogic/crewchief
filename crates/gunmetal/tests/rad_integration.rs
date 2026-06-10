//! RAD integration tests — full Radisk + FsStore stack on a real
//! filesystem (native only). Covers persistence across instances, chunk
//! splits, range queries spanning chunks, corrupt-file self-healing, and
//! GUN.js JSON-format directory compatibility.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use gunmetal::GunValue;
use gunmetal::rad::{
    FsStore, RadStorageAdapter, Radisk, RadiskOptions, Radix, ReadOpt, dename,
};
use gunmetal::storage::{StorageAdapter, StoredValue, storage_key};
use serde_json::json;

// ── Helpers ─────────────────────────────────────────────────────────

fn data_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("radata-{}", name));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn open(dir: &Path, opt: RadiskOptions) -> Radisk {
    let store = FsStore::new(dir.to_path_buf()).expect("create data dir");
    Radisk::new(Box::new(store), opt)
}

fn tiny_opts() -> RadiskOptions {
    RadiskOptions {
        chunk: 256,
        until_ms: 10_000, // tests flush explicitly
        ..Default::default()
    }
}

fn chunk_files(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|f| f != "%1C")
        .collect();
    files.sort();
    files
}

// ── Persistence across instances ────────────────────────────────────

#[test]
fn write_flush_reopen_read_back() {
    let dir = data_dir("reopen");
    let keys: Vec<String> = (0..200)
        .map(|i| format!("users/user{:03}\u{1B}score", i))
        .collect();

    {
        let rad = open(&dir, tiny_opts());
        for (i, key) in keys.iter().enumerate() {
            rad.put(key, json!(i), None).unwrap();
        }
        rad.flush().unwrap();
    } // dropped

    // Fresh instance over the same directory.
    let rad2 = open(&dir, tiny_opts());
    for (i, key) in keys.iter().enumerate() {
        assert_eq!(rad2.get(key).unwrap(), Some(json!(i)), "key {}", key);
    }

    // Full iteration is sorted and complete.
    let mut seen = Vec::new();
    rad2.each::<(), _>(&ReadOpt::default(), &mut |_, k| {
        seen.push(k.to_string());
        None
    })
    .unwrap();
    let mut expected = keys.clone();
    expected.sort();
    assert_eq!(seen, expected);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn drop_without_flush_still_persists() {
    let dir = data_dir("drop-flush");
    {
        let rad = open(&dir, tiny_opts());
        rad.put("alex", json!(27), None).unwrap();
        // No explicit flush — Drop flushes pending writes.
    }
    let rad2 = open(&dir, tiny_opts());
    assert_eq!(rad2.get("alex").unwrap(), Some(json!(27)));
    let _ = fs::remove_dir_all(&dir);
}

// ── Chunk splitting + cross-chunk ranges ────────────────────────────

#[test]
fn range_query_spans_chunk_split() {
    let dir = data_dir("split-range");
    {
        let rad = open(&dir, tiny_opts());
        for i in 0..100 {
            rad.put(&format!("item{:03}", i), json!(i), None).unwrap();
        }
        rad.flush().unwrap();
    }

    // The 256-byte chunk limit forced splits into multiple files.
    let chunks = chunk_files(&dir);
    assert!(
        chunks.len() > 1,
        "expected a chunk split, found only {:?}",
        chunks
    );
    assert!(chunks.contains(&"!".to_string()), "first chunk must be !");

    let rad2 = open(&dir, tiny_opts());

    // Forward range crossing chunk boundaries (inclusive bounds).
    let mut keys = Vec::new();
    rad2.each::<(), _>(
        &ReadOpt {
            start: Some("item020".into()),
            end: Some("item077".into()),
            ..Default::default()
        },
        &mut |v, k| {
            let i: u64 = k.trim_start_matches("item").parse().unwrap();
            assert_eq!(v, &json!(i));
            keys.push(k.to_string());
            None
        },
    )
    .unwrap();
    let expected: Vec<String> = (20..=77).map(|i| format!("item{:03}", i)).collect();
    assert_eq!(keys, expected);

    // Reverse iteration across the same span.
    let mut rev = Vec::new();
    rad2.each::<(), _>(
        &ReadOpt {
            start: Some("item020".into()),
            end: Some("item077".into()),
            reverse: true,
        },
        &mut |_, k| {
            rev.push(k.to_string());
            None
        },
    )
    .unwrap();
    let mut expected_rev = expected.clone();
    expected_rev.reverse();
    assert_eq!(rev, expected_rev);

    let _ = fs::remove_dir_all(&dir);
}

// ── Self-healing ────────────────────────────────────────────────────

#[test]
fn corrupt_chunk_file_self_heals() {
    let dir = data_dir("self-heal");
    {
        let rad = open(&dir, tiny_opts());
        for i in 0..100 {
            rad.put(&format!("item{:03}", i), json!(i), None).unwrap();
        }
        rad.flush().unwrap();
    }

    // Corrupt a non-root chunk file on disk.
    let chunks = chunk_files(&dir);
    let victim = chunks
        .iter()
        .find(|f| f.as_str() != "!")
        .expect("expected a non-root chunk")
        .clone();
    fs::write(dir.join(&victim), "{this is not json").unwrap();
    let victim_key = dename(&victim);

    // A read of a key in the corrupt chunk heals: the file is dropped
    // from the directory and the read returns None instead of erroring.
    let rad2 = open(&dir, tiny_opts());
    assert_eq!(rad2.get(&victim_key).unwrap(), None);

    // The directory on disk now marks the victim as deleted.
    let dir_raw = fs::read_to_string(dir.join("%1C")).unwrap();
    let dir_radix = Radix::from_json(&dir_raw).unwrap();
    assert_eq!(
        dir_radix.get(&victim_key),
        Some(gunmetal::rad::RadixGet::Leaf(json!(0)))
    );

    // Data in healthy chunks is still readable.
    assert_eq!(rad2.get("item000").unwrap(), Some(json!(0)));

    // And new writes for the healed range work again.
    rad2.put(&victim_key, json!("recovered"), None).unwrap();
    rad2.flush().unwrap();
    let rad3 = open(&dir, tiny_opts());
    assert_eq!(rad3.get(&victim_key).unwrap(), Some(json!("recovered")));

    let _ = fs::remove_dir_all(&dir);
}

// ── GUN.js JSON directory compatibility ─────────────────────────────

#[test]
fn reads_gun_js_written_data_directory() {
    let dir = data_dir("gun-compat");
    fs::create_dir_all(&dir).unwrap();

    // Hand-craft a data directory exactly as GUN.js writes it:
    // - chunk file `!` holding the nested radix JSON from rad.md
    // - directory file `\x1C` (URL-encoded `%1C`) listing the chunk
    fs::write(
        dir.join("!"),
        r#"{"a":{"lex":{"":27,"andria":{"":"library"}},"ndrew":{"":true}}}"#,
    )
    .unwrap();
    fs::write(dir.join("%1C"), r#"{"!":{"":1}}"#).unwrap();

    let rad = open(&dir, RadiskOptions::default());
    assert_eq!(rad.get("alex").unwrap(), Some(json!(27)));
    assert_eq!(rad.get("alexandria").unwrap(), Some(json!("library")));
    assert_eq!(rad.get("andrew").unwrap(), Some(json!(true)));
    assert_eq!(rad.get("missing").unwrap(), None);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn writes_gun_js_compatible_chunks() {
    let dir = data_dir("gun-compat-write");
    {
        let rad = open(&dir, RadiskOptions::default());
        rad.put("alex", json!(27), None).unwrap();
        rad.put("alexandria", json!("library"), None).unwrap();
        rad.put("andrew", json!(true), None).unwrap();
        rad.flush().unwrap();
    }

    // Chunk content is byte-for-byte the documented GUN format.
    assert_eq!(
        fs::read_to_string(dir.join("!")).unwrap(),
        r#"{"a":{"lex":{"":27,"andria":{"":"library"}},"ndrew":{"":true}}}"#
    );
    // Directory file is the radix JSON of {"!": 1}.
    assert_eq!(
        fs::read_to_string(dir.join("%1C")).unwrap(),
        r#"{"!":{"":1}}"#
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reads_gun_js_envelope_records() {
    // GUN's store.js persists `soul\x1Bkey` → {":": value, ">": state}.
    // Hand-craft such a chunk (states as integers, as JSON.stringify
    // emits them) and read it through the StorageAdapter bridge.
    let dir = data_dir("gun-envelope");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("!"),
        "{\"mark\\u001b\":{\"name\":{\"\":{\":\":\"Mark\",\">\":1700000000000}},\"boss\":{\"\":{\":\":{\"#\":\"fluffy\"},\">\":1700000000001}}}}",
    )
    .unwrap();
    fs::write(dir.join("%1C"), r#"{"!":{"":1}}"#).unwrap();

    let store = FsStore::new(dir.clone()).unwrap();
    let adapter = RadStorageAdapter::new(Radisk::with_store(Box::new(store)));

    let name = adapter
        .get(&storage_key("mark", "name").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(name.value, GunValue::Text("Mark".into()));
    assert_eq!(name.state, 1700000000000.0);

    let boss = adapter
        .get(&storage_key("mark", "boss").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(boss.value, GunValue::Link("fluffy".into()));

    let all = adapter.scan("mark\u{1B}").unwrap();
    assert_eq!(all.len(), 2);

    let _ = fs::remove_dir_all(&dir);
}

// ── StorageAdapter over the filesystem ──────────────────────────────

#[test]
fn storage_adapter_roundtrip_on_fs() {
    let dir = data_dir("adapter-fs");
    {
        let store = FsStore::new(dir.clone()).unwrap();
        let mut adapter = RadStorageAdapter::new(Radisk::new(
            Box::new(store),
            RadiskOptions {
                until_ms: 10_000,
                ..Default::default()
            },
        ));
        for i in 0..50 {
            adapter
                .put(
                    &storage_key("notes", &format!("n{:02}", i)).unwrap(),
                    &StoredValue {
                        value: GunValue::Number(i as f64),
                        state: 100.0 + i as f64,
                    },
                )
                .unwrap();
        }
        adapter.flush().unwrap();
    }

    let store = FsStore::new(dir.clone()).unwrap();
    let adapter = RadStorageAdapter::new(Radisk::with_store(Box::new(store)));
    let all = adapter.scan("notes\u{1B}").unwrap();
    assert_eq!(all.len(), 50);
    for (i, (k, v)) in all.iter().enumerate() {
        assert_eq!(k, &format!("notes\u{1B}n{:02}", i));
        assert_eq!(v.value, GunValue::Number(i as f64));
        assert_eq!(v.state, 100.0 + i as f64);
    }

    let _ = fs::remove_dir_all(&dir);
}

// ── Timer-based flush over the filesystem ───────────────────────────

#[test]
fn timer_flush_persists_without_explicit_flush() {
    let dir = data_dir("timer");
    let rad = open(
        &dir,
        RadiskOptions {
            until_ms: 25,
            ..Default::default()
        },
    );
    rad.put("alex", json!(27), None).unwrap();

    // Wait (generously) for the 25ms batch timer.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !dir.join("!").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        fs::read_to_string(dir.join("!")).unwrap(),
        r#"{"alex":{"":27}}"#
    );

    let _ = fs::remove_dir_all(&dir);
}

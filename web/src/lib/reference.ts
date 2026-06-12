/**
 * Per-module reference items (spec §3.5): signature, params/defaults,
 * returns, the wasm/JS-bound name where one exists, examples, caveats.
 * Keyed by module name from the catalog manifest. Modules without an
 * entry render purpose + demos only.
 */

export interface RefItem {
	/** Rust item path, e.g. "GunChain::put_kv". */
	name: string;
	/** Rust signature (condensed). */
	signature: string;
	/** JS-bound name on the wasm surface, when one exists. */
	wasmName?: string;
	/** Parameters and defaults, one line. */
	params?: string;
	returns?: string;
	exampleRust?: string;
	exampleJs?: string;
	caveats?: string[];
}

export const referenceItems: Record<string, RefItem[]> = {
	instance: [
		{
			name: 'Gun::new',
			signature: 'pub fn new(options: GunOptions) -> Gun',
			wasmName: 'new WasmGun() / WasmGun.withOptions(json)',
			params:
				'GunOptions: peers=[], file="radata", local_storage=true, radisk=true, axe=true, ws_path="/gun", chunk=1MiB, mob=999999, gap=0ms, retry=60, pid=random',
			returns: 'A Gun instance (cheaply cloneable; clones share state)',
			exampleRust: 'let gun = Gun::new(GunOptions::default());',
			exampleJs: `const gun = WasmGun.withOptions(JSON.stringify({ localStorage: false }));`
		},
		{
			name: 'GunChain::get',
			signature: 'pub fn get(&self, key: &str) -> GunChain',
			wasmName: 'gun.get(soul, key) (resolved read), gun.getNode(soul)',
			returns: 'A chain scoped one level deeper (soul → key)',
			exampleRust: 'gun.get("people/ada").get("name")'
		},
		{
			name: 'GunChain::put_kv',
			signature: 'pub fn put_kv(&self, key: impl Into<String>, value: GunValue) -> &Self',
			wasmName: 'putText / putNumber / putBool / putNull / putLink / putObject',
			returns: 'The chain (for chaining); events fire after HAM accepts',
			exampleRust: 'gun.get("people/ada").put_kv("name", GunValue::Text("Ada".into()));',
			exampleJs: `gun.putText('people/ada', 'name', 'Ada');`
		},
		{
			name: 'GunChain::val',
			signature: 'pub fn val(&self) -> Option<GunValue>',
			wasmName: 'gun.get(soul, key)',
			returns: 'Current local value (no network wait)',
			caveats: ['Reads only what this peer has merged — pair with fetchSoul/on for remote state.']
		},
		{
			name: 'GunChain::on',
			signature: 'pub fn on(&self, cb: impl FnMut(GunValue, String) + MaybeSend + \'static) -> ListenerId',
			wasmName: 'gun.on(soul, key, cb) / gun.onNode(soul, cb) → id; gun.off(soul, key, id)',
			returns: 'ListenerId for off()',
			caveats: [
				'Node-level subscriptions fire PER KEY with (value, key) — not with the whole node.',
				'Subscribing does not replay existing state on its own: read current values explicitly after subscribing.',
				'Callbacks may write and subscribe (snapshot-and-release emission); a listener writing its own tag is skipped on the re-entrant fire.'
			]
		},
		{
			name: 'GunChain::map',
			signature: 'pub fn map(&self, lex: Option<&Lex>, cb: impl FnMut(GunValue, String)) -> ListenerId',
			params: 'lex: optional LEX filter over keys',
			returns: 'Fires for existing pairs and future changes',
			caveats: ['Auto-pins the soul against LRU eviction while subscribed.']
		},
		{
			name: 'GunChain::set / set_value',
			signature: 'pub fn set(&self, item: GunChain) -> GunChain / pub fn set_value(&self, value: GunValue) -> String',
			wasmName: 'setObject(setSoul, json) → itemSoul / setValue(setSoul, json) → key',
			returns: 'The item chain / the generated time-sortable uuid key',
			caveats: ['Keys are time-sortable: sorting keys = insertion order, no clock coordination.']
		}
	],

	mesh: [
		{
			name: 'Mesh::new',
			signature: 'pub fn new(gun: Gun, config: MeshConfig) -> Mesh',
			wasmName: 'implicit — created by WasmGun.connect()',
			params: 'MeshConfig::from_options(&GunOptions): pid, gap, puff=9, seen_by caps (7 peers / 99 chars), ack_puts=true, axe',
			caveats: ['Registers a "put" listener that broadcasts local writes; create at most one Mesh per Gun.']
		},
		{
			name: 'Mesh::hi / bye',
			signature: 'pub fn hi(&self, peer_id: &str, url: Option<String>, sender: Option<PeerSender>) / pub fn bye(&self, peer_id: &str)',
			wasmName: 'gun.connect(url) / gun.disconnect(url)',
			returns: 'hi sends the DAM "?" handshake; bye applies registered disconnect writes',
			caveats: [
				'Same-direction duplicate connections (same pid) resolve to the NEWER link — a reconnect supersedes its dead socket.',
				'Mixed-direction duplicates: the link dialed by the higher-pid side survives, on both ends.'
			]
		},
		{
			name: 'Mesh::hear / hear_async',
			signature: 'pub fn hear(&self, raw: &str, from_peer: &str) / pub async fn hear_async(...)',
			returns: 'Parses, dedups, dispatches; hear_async yields every `puff` (9) messages',
			caveats: [
				'Frames over 10 MB are dropped whole; one malformed element in a batch drops only that element.',
				'`##`+`@` dedup recomputes the content hash locally — a forged `##` cannot suppress a genuine answer (intentional spec deviation).'
			]
		},
		{
			name: 'Mesh::say / flush',
			signature: 'pub fn say(&self, msg: WireMessage, to_peer: Option<&str>) / pub fn flush(&self)',
			wasmName: 'gun.fetchSoul(soul) (an outgoing GET) / gun.flushMesh()',
			returns: 'ACKs route to the original requester via the dup table; data broadcasts (AXE: to subscribers)',
			caveats: ['With gap > 0, messages batch into one JSON array per window; flush() forces it.']
		},
		{
			name: 'WasmGun networking surface',
			signature: 'connect(url) / disconnect(url) / isConnected(url) / connectedPeers() / peerPid(url) / onStatus(cb) / onWire(cb)',
			wasmName: 'same',
			returns: 'peerPid(url) turning non-null is the handshake-acked signal; onWire taps (direction, peer, raw) for every frame',
			exampleJs: `gun.onWire((dir, peer, raw) => console.log(dir, raw));\ngun.connect('ws://localhost:8765/gun');`
		}
	],

	relay: [
		{
			name: 'gunmetal-relay (binary)',
			signature: 'cargo run -p gunmetal --features relay --bin gunmetal-relay -- [flags]',
			params:
				'--port 8765 | PORT · --host 0.0.0.0 | HOST · --path /gun | GUN_PATH · --file radata | GUN_FILE · --mob N | MOB · --peer URL (repeatable) | GUN_PEERS · --tls-cert/--tls-key (relay-tls feature)',
			returns: 'WebSocket relay wire-compatible with GUN.js clients; /health returns {"ok":true,"peers":n,"pid":..}',
			caveats: ['CLI flags override env vars override defaults.']
		},
		{
			name: 'relay::spawn / spawn_in_memory',
			signature: 'pub async fn spawn(config: RelayConfig) -> Result<RelayHandle, String>',
			returns: 'spawn persists via RAD to the configured dir; spawn_in_memory forgets on shutdown (tests)',
			caveats: [
				'Over-capacity (mob) connections are shed with a {dam:"mob"} redirect listing upstream peers.',
				'HTTP head reads are byte-budgeted (8 KB) and time-budgeted (10 s); oversized messages rejected at the WebSocket layer (10 MB).',
				'Bye-write registrations are capped per peer by count (100) and bytes (1 MB).'
			]
		}
	],

	extended: [
		{
			name: 'GunChain::path',
			signature: "pub fn path(&self, path: &str) -> GunChain",
			wasmName: 'pathVal(soul, "a.b.c")',
			returns: 'Chain after splitting on "." — path("a.b") == get("a").get("b")'
		},
		{
			name: 'GunChain::open',
			signature: 'pub fn open(&self, options: OpenOptions, cb) -> OpenHandle',
			params: 'OpenOptions: wait=9ms debounce, depth=None, once=false, meta=false',
			returns: 'Assembled document on every change at any depth; OpenHandle::off() (also on Drop) unsubscribes',
			caveats: [
				'Cycles surface as {"#": soul} markers; depth limit truncates the same way.',
				'Reassembles the full document per delivery — avoid on a relay\'s hot write path.'
			]
		},
		{
			name: 'GunChain::load',
			signature: 'pub fn load(&self, cb: impl FnOnce(Value))',
			wasmName: 'load(soul, cb)',
			returns: 'open() with once: the full tree, exactly one callback'
		},
		{
			name: 'GunChain::not / not_within',
			signature: 'pub async fn not_within(&self, timeout: Duration, cb: impl FnOnce(&str))',
			wasmName: 'notWithin(soul, key, ms) → Promise<bool>',
			returns: 'Fires when nothing was found in time',
			caveats: ['CANNOT guarantee absence in a distributed system — an unmet peer may hold the data. Treat as "nothing found here, yet".']
		},
		{
			name: 'GunChain::unset',
			signature: 'pub fn unset(&self, item: &GunChain) -> &Self',
			wasmName: 'unset(setSoul, itemSoul)',
			returns: 'Nulls the link key in the set',
			caveats: ['Removes the LINK, not the item node — the target survives and other references remain valid.']
		},
		{
			name: 'GunChain::then / promise',
			signature: 'pub async fn then(&self) -> Option<GunValue>',
			wasmName: 'once(soul, key) → Promise',
			caveats: ['Exported to JS as `once`, NOT `then` — a `then` method would make the object a thenable and corrupt `await`.']
		},
		{
			name: 'GunChain::later',
			signature: 'pub async fn later(&self, delay: Duration, cb)',
			returns: 'Fires once after the delay with a full-depth snapshot (via open)'
		},
		{
			name: 'GunChain::bye',
			signature: 'pub fn bye(&self) -> ByeBuilder; ByeBuilder::put(value) -> WireMessage',
			wasmName: 'registerBye(soul, key, json)',
			returns: 'A registration the RELAY applies when this peer disconnects',
			caveats: [
				'Requires relay support; experimental in GUN itself.',
				'Bye writes cannot target ~user namespaces (they are unsigned).'
			]
		}
	],

	sea: [
		{
			name: 'sea::pair',
			signature: 'pub fn pair() -> Result<SEAPair, SeaError>',
			wasmName: 'sea.pair() → {pub, priv, epub, epriv} JSON',
			returns: 'P-256 signing pair + encryption pair'
		},
		{
			name: 'sea::sign / verify',
			signature: 'pub fn sign(data, priv, pub) -> Result<String> / pub fn verify(message, pub) -> Result<Value>',
			wasmName: 'sea.sign(data, priv, pub) / sea.verify(message, pub)',
			returns: 'verify returns the signed payload; it ERRORS on a bad signature',
			caveats: ['Pure-Rust crypto (no WebCrypto path) — same algorithms as GUN (ECDSA P-256).']
		},
		{
			name: 'sea::encrypt / decrypt',
			signature: 'pub fn encrypt(data, key) -> Result<String> / pub fn decrypt(message, key) -> Result<String>',
			wasmName: 'sea.encrypt(data, key) / sea.decrypt(ct, key)',
			returns: 'AES-GCM ciphertext envelope / the plaintext',
			caveats: ['decrypt errors on a wrong key — catch it; that IS the wrong-passphrase signal.']
		},
		{
			name: 'sea::work',
			signature: 'pub fn work(data: &str, salt: Option<&str>) -> Result<String>',
			wasmName: 'sea.work(data, salt?)',
			returns: 'PBKDF2-derived key (deliberately slow)',
			caveats: ['Omitting the salt generates a RANDOM one — pass a fixed salt when two parties must derive the same key.']
		},
		{
			name: 'sea::secret',
			signature: 'pub fn secret(their_epub: &str, my_epriv: &str) -> Result<String>',
			wasmName: 'sea.secret(theirEpub, myEpriv)',
			returns: 'ECDH shared key: secret(their_epub, my_epriv) == secret(my_epub, their_epriv)'
		}
	],

	user: [
		{
			name: 'User::create',
			signature: 'pub fn create(&mut self, alias: &str, password: &str) -> CreateResult',
			wasmName: 'user.create(alias, password) → {ok, pub} | {err}',
			returns: 'Generates a pair, encrypts private halves with a password-derived proof, stores in the graph',
			caveats: ['There is no password reset — the keys ARE the account.']
		},
		{
			name: 'User::auth_with_password / auth_with_pair',
			signature: 'pub fn auth_with_password(&mut self, alias, password) -> AuthResult / pub fn auth_with_pair(&mut self, pair: SEAPair) -> AuthResult',
			wasmName: 'user.auth(alias, password) / user.authPair(pairJson)',
			returns: 'Decrypts (or accepts) the pair; subsequent user writes are signed',
			exampleJs: `// session restore, no password round-trip:\nsessionStorage.setItem(k, user.pairJson());\nuser.authPair(sessionStorage.getItem(k));`
		},
		{
			name: 'User::get / SignedChain',
			signature: 'pub fn get(&self, key) -> Option<GunChain> / get_signed(&self, key) -> Option<SignedChain>',
			wasmName: 'user.put / user.get / user.putSigned / user.getSigned',
			returns: 'Chains under ~<pub> — every peer verifies signatures before merging writes there',
			caveats: ['Alias registration races are unresolvable without consensus — first-write-wins per HAM (documented limitation).']
		},
		{
			name: 'User::leave',
			signature: 'pub fn leave(&mut self)',
			wasmName: 'user.leave()',
			returns: 'Drops the in-memory pair (logout)'
		}
	],

	cert: [
		{
			name: 'Certificate::create',
			signature: 'pub fn create(who: CertWho, what: CertWhat, expiry: Option<f64>, issuer_pub, issuer_priv) -> Result<Certificate>',
			wasmName: 'cert.create(who | "*", path | "prefix*" | "*", expiry?, issuerPub, issuerPriv)',
			returns: 'A grant signed by the issuer: who may write where, until when'
		},
		{
			name: 'Certificate::verify / grants_access',
			signature: 'pub fn verify(&self) -> Result<bool> / pub fn grants_access(&self, writer_pub, path, now_ms) -> bool',
			wasmName: 'cert.verify(certJson) / cert.grantsAccess(certJson, writerPub, path)',
			returns: 'Signature validity / the full scope+expiry+identity check',
			caveats: [
				'Enforcement is READ-time, by every reader, with math — no server blocks an uncertified write; readers refuse to believe it.',
				'Revocation is a tombstone the owner publishes; readers must check for it.'
			]
		}
	],

	wire: [
		{
			name: 'WireMessage',
			signature: 'pub struct WireMessage { id "#", ack "@", put, get, hash "##", seen_by "><", dam, pid, ok, bye, .. }',
			returns: 'The GUN wire format: JSON messages, optionally batched into arrays',
			exampleJs: `{"#":"msg-id","put":{"soul":{"_":{"#":"soul",">":{"k":171…}},"k":"v"}}}`
		},
		{
			name: 'put_message / get_message / parse_message',
			signature: 'pub fn put_message(id, nodes: &[&Node]) -> WireMessage / get_message(id, soul, key) / parse_message(raw) -> Result<WireMessage>',
			caveats: [
				'Messages over MAX_MESSAGE_BYTES (10 MB) are rejected with an explicit size error.',
				'`><` (seen_by) is REPLACED at each relay hop, capped at 7 peers / 99 chars.',
				'Heartbeats are the empty array `[]` every 20 s — never processed as data.'
			]
		}
	],

	rad: [
		{
			name: 'Radix',
			signature: 'pub fn insert(&mut self, key, value) / get(&self, key) -> Option<RadixGet> / map(&self, opt, cb)',
			returns: 'In-memory radix tree, keys soul+\\x1B+key, values {":": value, ">": state} envelopes',
			caveats: ['Depth is capped at 100 (coordinated with serde_json\'s parse recursion limit) — inserts at the cap are rejected so every stored tree round-trips.']
		},
		{
			name: 'Radisk',
			signature: 'chunked persistence: 1 MiB chunks, 250 ms write batching, atomic temp-file+rename writes',
			returns: 'JSON serialization byte-compatible with GUN.js radata directories',
			caveats: ['Storage-dir compatibility with GUN.js is best-effort (JSON-mode chunks; legacy binary format is read-only territory).']
		},
		{
			name: 'FsStore / IdbStore',
			signature: 'native filesystem store / browser IndexedDB store',
			wasmName: 'gun.enablePersistence(dbName) wires IndexedDB end-to-end',
			caveats: [
				'enablePersistence hydrates with ORIGINAL HAM states through the normal merge path.',
				'Persistence is per dbName: same-origin contexts sharing a name share data — namespace per logical session.'
			]
		}
	],

	storage: [
		{
			name: 'StorageAdapter / AsyncStorageAdapter',
			signature: 'pub trait StorageAdapter: MaybeSend { fn put/get/delete/list } (+ async variant)',
			returns: 'Pluggable persistence backends (memory, fs, IndexedDB, yours)'
		},
		{
			name: 'BatchWriter',
			signature: 'pub fn new(adapter, BatchConfig) ; buffer_put(key, value)',
			params: 'BatchConfig: flush interval + max buffered count',
			returns: 'Write coalescing for chatty workloads'
		},
		{
			name: 'StorageEngine',
			signature: 'hydrate-at-startup + persist-on-put wrapper used by the relay',
			caveats: ['The relay persists through this; in-memory relays (tests) forget on restart by design.']
		}
	],

	lex: [
		{
			name: 'Lex',
			signature: 'Lex::exact(v) / Lex::prefix(v) / Lex::range(gte, lte)',
			returns: 'Key matchers for map() and queries',
			exampleRust: 'chain.map(Some(&Lex::prefix("2026-")), |v, k| { .. });',
			caveats: ['Set keys are time-sortable uuids, so a key RANGE is a time range — that\'s the pagination idiom.', 'Wire-level LEX GETs are not implemented; filter client-side over synced data.']
		}
	],

	dup: [
		{
			name: 'Dup',
			signature: 'pub fn check(&mut self, id) -> bool / track(id) / track_from(id, via) / via(id) -> Option<String>',
			returns: 'Message dedup with via-tracing: ACKs route back along the path the request came from',
			caveats: ['Entries expire after the TTL (9 s, GUN-compatible) — ACKs arriving later fall back to broadcast.', 'Bounded: oldest entries evict O(1) past the size cap.']
		}
	],
	types: [
		{ name: 'GunValue', signature: 'pub enum GunValue { Null, Bool(bool), Number(f64), Text(String), Link(Soul) }', returns: 'The five wire-representable value kinds; Link carries a target soul' },
		{ name: 'Node', signature: 'pub struct Node { soul, values, states }; Node::new(soul) / put(key, value, state) / get(key) / state_of(key) / iter()', returns: 'A flat record with a HAM state per key' },
		{ name: 'Soul', signature: 'pub type Soul = String', returns: 'The globally-unique node id' }
	],

	graph: [
		{ name: 'Graph', signature: 'pub fn put_node(&mut self, soul, data) -> Vec<(String, PutResult)> / get(&self, soul, key) / get_node(&self, soul)', returns: 'The in-memory store; put runs HAM per key and reports Accepted/Rejected' },
		{ name: 'Graph::pin / unpin', signature: 'pub fn pin(&mut self, soul) / unpin(&mut self, soul)', returns: 'Exempt a soul from LRU eviction (subscriptions auto-pin)', caveats: ['Eviction only drops UNPINNED nodes; a long-lived subscription set bounds what can be evicted.'] }
	],

	crdt: [
		{ name: 'ham', signature: 'pub fn ham(machine_state, incoming_state, current_state, incoming_value, current_value) -> HamResult', returns: 'The conflict verdict: take incoming, keep current, defer (future state), or tie-break lexically on value', caveats: ['Future states beyond the drift cap are deferred, defanging lying clocks.', 'Equal state + equal value = no-op (no event fires).'] }
	],

	state: [
		{ name: 'now_ms', signature: 'pub fn now_ms() -> f64', returns: 'Wall-clock milliseconds — the state source for writes (cross-platform)' },
		{ name: 'State', signature: 'pub struct State — per-key HAM timestamps on a node', returns: 'The `>` map in wire frames' }
	],

	sync: [
		{ name: 'SyncPair / MemorySync', signature: 'two-instance in-memory sync harness', returns: 'Test-oriented direct sync without sockets; superseded by Mesh for real transports', caveats: ['Kept for simple two-peer testing (spec §2.8); Mesh implements AsyncSyncAdapter as the drop-in replacement.'] }
	],

	transport: [
		{ name: 'WsNativeTransport', signature: 'tokio-tungstenite client/server WebSocket transport (native)', returns: 'Sinks/streams with a 10 MB message cap and bounded channels' },
		{ name: 'WsWasmTransport', signature: 'web_sys::WebSocket transport (browser): connect(url) / send / close / set_on_message / set_on_open / set_on_close / set_on_error', wasmName: 'driven internally by WasmGun.connect()', caveats: ['Handlers are detached before closures drop on close/replace — the browser fires close asynchronously.'] },
		{ name: 'reconnect', signature: 'exponential backoff with jitter', returns: 'Shared by native and wasm reconnection logic' }
	],

	events: [
		{ name: 'EventBus', signature: 'pub fn on(tag, cb) -> ListenerId / once(tag, cb) / off(tag, id) / emit(tag, event)', returns: 'The pub-sub backbone (tags: put, get, hi, bye, soul#key)' },
		{ name: 'emit_unlocked', signature: 'pub fn emit_unlocked(bus, tag, event)', returns: 'Snapshot-and-release emission: listeners run with NO bus lock held, so they may subscribe/unsubscribe/write freely', caveats: ['Self-recursive fires are skipped; cross-thread fires block (never dropped).', 'off() does not cancel in-flight snapshot deliveries.'] }
	],

	uuid: [
		{ name: 'generate_uuid', signature: 'pub fn generate_uuid() -> String', wasmName: 'generateUUID()', returns: 'Time-sortable unique id (timestamp prefix + randomness) — set keys sort in insertion order' },
		{ name: 'random_message_id', signature: 'pub fn random_message_id(len: usize) -> String', returns: 'Wire message ids (`#`)' }
	],

	concurrency: [
		{ name: 'SharedMut / lock_mut', signature: 'type SharedMut<T> = Arc<Mutex<T>> (native) | Rc<RefCell<T>> (wasm)', returns: 'The platform-gated shared-state primitive everything builds on' },
		{ name: 'MaybeSend', signature: 'trait MaybeSend (Send supertrait on native; blanket no-op on wasm)', returns: 'Bound for callbacks that must be Send on native but JsValue-capturing on wasm' }
	],

	runtime: [
		{ name: 'spawn / sleep_async', signature: 'pub fn spawn(fut) / pub async fn sleep_async(d: Duration)', returns: 'tokio on native, wasm_bindgen_futures + setTimeout on wasm' }
	],

	wasm: [
		{ name: 'init / module boot', signature: "import init, { WasmGun, WasmSEA, WasmUser, WasmCert } from './gunmetal.js'; await init()", wasmName: 'the module itself', returns: 'wasm-bindgen web-target bundle; built by web/scripts/build-wasm.sh (CLI version locked to Cargo.lock)' },
		{ name: 'WasmGun', signature: 'the JS facade over Gun + Mesh + transport + persistence', wasmName: 'WasmGun', returns: 'Every per-item JS name is listed on its owning module page (instance, mesh, extended, sea, user, cert, storage)', caveats: ['Promise-returning read is exported as once, not then — a then method would make the object a thenable and corrupt await.'] }
	]
};

/**
 * Per-demo page content (spec §4 b/c/e): why the pattern exists, minimal
 * copy-paste snippets (runnable against a local relay — no site glue),
 * and gotchas. Keyed by demo slug.
 */

export interface DemoSnippet {
	label: string;
	code: string;
}

export interface DemoContent {
	/** (b) The real-world need this capability answers. */
	why: string;
	/** (c) Minimal per-operation snippets, copy-paste-true. */
	snippets: DemoSnippet[];
	/** (e) Gotchas & limits, stated plainly. */
	gotchas: string[];
}

export const demoContent: Record<string, DemoContent> = {
	'graph-explorer': {
		why: 'Every database starts with "put something in, get it back". The graph model — flat nodes addressed by souls, composed with links — is what makes the rest of gunmetal (sync, merge, signatures) possible, so it deserves to be poked at directly before any networking is involved.',
		snippets: [
			{
				label: 'Write, read, link',
				code: `const gun = new WasmGun();
gun.putText('people/mark', 'name', 'Mark');
gun.get('people/mark', 'name');            // '"Mark"' (JSON)
gun.putLink('index', 'mark', 'people/mark'); // a soul-to-soul pointer`
			},
			{
				label: 'Subscribe to a node',
				code: `gun.onNode('people/mark', (valueJson, key) => {
  // fires PER CHANGED KEY: (value, key)
  console.log(key, '→', JSON.parse(valueJson));
});`
			}
		],
		gotchas: [
			'Node subscriptions fire per key with (value, key) — not with the whole node. Re-read getNode(soul) if you want the full record.',
			'Subscriptions deliver changes, not history: read existing state explicitly after subscribing if the data may already be there.'
		]
	},
	'shared-input': {
		why: 'Collaborative state — a form two people edit, a setting that syncs across your own devices — is the canonical reason realtime sync exists. One put on one peer, one callback on every other peer.',
		snippets: [
			{
				label: 'Connect and sync one field',
				code: `const gun = WasmGun.withOptions(JSON.stringify({ localStorage: false }));
gun.connect('ws://localhost:8765/gun');

gun.on('doc/title', 'text', (json) => render(JSON.parse(json)));
gun.fetchSoul('doc/title');               // pull current state
input.oninput = () => gun.putText('doc/title', 'text', input.value);`
			}
		],
		gotchas: [
			'peerPid(url) turning non-null is the "handshake done" signal — writes before that only live locally until the connection settles.',
			'fetchSoul() is how a late joiner pulls existing state; subscribing alone does not request data from peers.'
		]
	},
	'chat-room': {
		why: "Append-only feeds (chat, comments, activity logs) can't use array indexes — two peers appending concurrently would fight over list[3]. Sets with time-sortable generated keys make every append conflict-free and chronologically sortable without a shared clock.",
		snippets: [
			{
				label: 'Append + render in order',
				code: `gun.setValue('rooms/lobby', JSON.stringify('alice|hello'));

gun.onNode('rooms/lobby', (json, key) => {
  messages[key] = JSON.parse(json);
  render(Object.entries(messages).sort(([a], [b]) => a < b ? -1 : 1));
});`
			}
		],
		gotchas: [
			'Keys are time-sortable uuids — sort the KEYS, not arrival order (network reordering is normal).',
			'History pagination here filters client-side over the synced set; wire-level LEX queries are future crate work.'
		]
	},
	'todo-list': {
		why: 'Shared mutable collections — task lists, carts, playlists — need add, edit, and remove that merge cleanly across peers. Items as linked nodes give per-field edits; unset() gives removal that other peers converge on.',
		snippets: [
			{
				label: 'Add an item node to a set',
				code: `const itemSoul = gun.setObject('todos/house',
  JSON.stringify({ text: 'buy milk', done: false }));`
			},
			{
				label: 'Toggle and remove',
				code: `gun.putBool(itemSoul, 'done', true);
gun.unset('todos/house', itemSoul);   // nulls the LINK in the set`
			}
		],
		gotchas: [
			'unset() removes the link, not the item node — anyone holding the soul can still read it (deliberate GUN semantic).',
			'When you discover an item via its set link, its field data may have arrived first — read getNode(itemSoul) after subscribing or you will miss it.'
		]
	},
	presence: {
		why: '"Who\'s here right now" powers avatars, typing indicators, and editor cursors. Heartbeat souls give liveness; a bye() registration makes departure visible immediately — the relay writes it the moment the socket drops, no timeout wait.',
		snippets: [
			{
				label: 'Heartbeat + last-seen',
				code: `setInterval(() =>
  gun.putNumber('room/presence', myId, Date.now()), 2000);

// the RELAY writes this for us when we disconnect:
gun.registerBye('room/left', myId, 'true');`
			}
		],
		gotchas: [
			'Derive presence from room-scoped heartbeat souls, not from mesh peer lists — peers of the relay are not members of your room.',
			'registerBye requires an active connect() and the value is fixed at registration time (the relay cannot compute timestamps for you).',
			'bye() writes are unsigned and therefore rejected for ~user namespaces.'
		]
	},
	'conflict-lab': {
		why: 'Offline editing is not an edge case — it is the normal state of a distributed app. HAM is the answer to "both of us edited the same field while apart": one winner, chosen identically on every peer, with no referee.',
		snippets: [
			{
				label: 'Split, edit, heal',
				code: `gun.disconnect(relay);          // edits now stay local
gun.putText('doc', 'text', 'my offline edit');

gun.connect(relay);             // reconnect…
// re-announce local edits, then pull:
gun.putText('doc', 'text', current);
gun.fetchSoul('doc');           // HAM converges both sides`
			}
		],
		gotchas: [
			'Convergence, not correctness: HAM picks A winner deterministically — it cannot know which edit was "right".',
			'Only re-announce values actually edited while offline; replaying stale state with a fresh timestamp beats real edits.'
		]
	},
	login: {
		why: 'Accounts without an account database: identity is a keypair, "signing up" derives and encrypts it with your password, and any peer can store the encrypted blob without being able to use it. Writes to your namespace are signed and verified by everyone.',
		snippets: [
			{
				label: 'Create, auth, write signed',
				code: `const user = new WasmUser(gun);
user.create('ada', 'correct horse battery');
user.auth('ada', 'correct horse battery');
user.put('bio', JSON.stringify('mathematician')); // → ~<pub>/bio`
			},
			{
				label: 'Session restore (no password)',
				code: `const pair = user.pairJson();          // {pub, priv, epub, epriv}
sessionStorage.setItem('me', pair);    // your risk to own
// later:
user.authPair(sessionStorage.getItem('me'));`
			}
		],
		gotchas: [
			'There is no password reset — the keys ARE the account. Lose both the password and the pair and the namespace is orphaned.',
			'pairJson() hands you private keys; where you persist them is the entire security model of your app.',
			'Aliases are global to the mesh — first writer wins a name.'
		]
	},
	'private-notes': {
		why: 'The graph replicates to every interested peer — relays included. Anything that must stay private has to be ciphertext before it is ever put. A passphrase-derived key (slow PBKDF2) lets a group share access without sharing accounts.',
		snippets: [
			{
				label: 'Derive, encrypt, store',
				code: `const sea = new WasmSEA();
const key = sea.work('passphrase', 'fixed-salt');  // SAME salt everywhere
const ct  = sea.encrypt(JSON.stringify(note), key);
gun.setValue('notes', JSON.stringify(ct));`
			}
		],
		gotchas: [
			'work() with no salt generates a RANDOM one — two sessions will derive different keys. Pass a fixed salt for shared spaces.',
			'Encryption hides content, not activity: souls, keys, timing, and sizes remain visible to every peer.'
		]
	},
	'secret-handshake': {
		why: 'Two people DMing should not need a pre-shared password. ECDH lets each side derive the SAME secret from their own private key plus the other\'s public key — an observer holding both public keys derives nothing.',
		snippets: [
			{
				label: 'Derive the shared secret',
				code: `// publish only myPair.epub; then:
const shared = sea.secret(their_epub, myPair.epriv);
const ct = sea.encrypt(JSON.stringify('psst'), shared);
// receiver: sea.secret(sender_epub, my_epriv) → the SAME key`
			}
		],
		gotchas: [
			'secret() uses the encryption keypair (epub/epriv), not the signing pair (pub/priv).',
			'Encrypting proves nothing about WHO sent it — add sign()/verify() if authenticity matters.'
		]
	},
	'doc-permissions': {
		why: 'Sharing write access without sharing keys and without a server ACL: the owner signs a small grant ("this key may write here"), writers attach it, and every reader independently verifies grant + signature. Enforcement is math at read time.',
		snippets: [
			{
				label: 'Grant, write, verify',
				code: `// owner:
const grant = cert.create(guestPub, 'shared-doc', undefined, myPub, myPriv);
// guest attaches it to a signed entry:
const sig = sea.sign(JSON.stringify(text), guestPriv, guestPub);
// EVERY reader checks:
cert.grantsAccess(grant, writerPub, 'shared-doc') &&
sea.verify(sig, writerPub);`
			}
		],
		gotchas: [
			'Uncertified writes are not blocked from existing — they are simply not BELIEVED by anyone who checks. Readers must actually check.',
			'Expiry lives inside the signed grant; revocation is a tombstone the owner publishes — readers must look for it.'
		]
	},
	'profile-tree': {
		why: 'Real records are nested — a profile with an address with a city — but graphs store flat nodes. load()/open() walk the links and hand back the assembled document, so you keep per-node sync AND document-shaped reads.',
		snippets: [
			{
				label: 'Build with links, read as one document',
				code: `gun.putText('profile/ada', 'name', 'Ada');
gun.putText('profile/ada/address', 'city', 'London');
gun.putLink('profile/ada', 'address', 'profile/ada/address');

gun.load('profile/ada', (json) => {
  // { name: "Ada", address: { city: "London" } }
});`
			},
			{
				label: 'Honest empty states',
				code: `const absent = await gun.notWithin('profile/nobody', '', 400);
if (absent) showEmptyState();`
			}
		],
		gotchas: [
			'not() means "nothing found here, yet" — absence can never be guaranteed in a distributed system.',
			'open()/load() handle cycles, but unbounded depth on big graphs is expensive — use the depth limit for documents you don\'t control.'
		]
	},
	'offline-first': {
		why: 'An app that only works online is a thin client. Local persistence plus merge-on-reconnect means the app is ALWAYS usable — the network is an optimization, not a requirement.',
		snippets: [
			{
				label: 'Persist locally, then go online',
				code: `const gun = WasmGun.withOptions(JSON.stringify({ localStorage: false }));
await gun.enablePersistence('my-app');   // IndexedDB; hydrates first
gun.connect(relay);                      // sync whenever possible`
			}
		],
		gotchas: [
			'Persistence is per database NAME — same-origin contexts (iframes, tabs) sharing a name share data. Namespace per logical session.',
			'Hydration replays stored values with their ORIGINAL HAM states, so local history merges with remote updates instead of overwriting.'
		]
	},
	'wire-inspector': {
		why: '"Why isn\'t my put syncing?" is unanswerable from app state alone. Watching the actual frames — handshake, put, the relay\'s ack, the peer\'s incoming put — turns sync debugging from guesswork into reading.',
		snippets: [
			{
				label: 'Tap the wire',
				code: `gun.onWire((direction, peer, raw) => {
  console.log(direction === 'in' ? '◀' : '▶', raw);
});
gun.connect(relay);   // register BEFORE connect to see the handshake`
			}
		],
		gotchas: [
			'No ack for your put usually means the relay never accepted it; an ack with no peer delivery points at subscriptions (did the peer GET that soul?).',
			'A lone "[]" every ~20s is the heartbeat — normal, not data.'
		]
	},
	benchmark: {
		why: 'Claims about speed are worthless without measurement on YOUR machine. Identical workloads run on both engines side by side: graph CPU (puts, subscription fires), SEA crypto (where GUN.js uses native WebCrypto and may well win), and relay round-trips. An instrument, not a scoreboard.',
		snippets: [
			{
				label: 'Time any workload',
				code: `const start = performance.now();
for (let i = 0; i < 3000; i++) gun.putText('bench', 'k' + i, 'v' + i);
console.log(3000 / ((performance.now() - start) / 1000), 'ops/s');`
			}
		],
		gotchas: [
			'The wasm bundle is built with opt-level=z (size-optimized): gunmetal CPU numbers here are conservative versus an -O3 build.',
			'GUN.js SEA rides native WebCrypto: expect it to win sign/verify/PBKDF2 by ~10x, and expect gunmetal to win AES encrypt/decrypt (WebCrypto pays per-call async overhead). Both are honest results.',
			"The put loops measure different pipelines: gun.js defers most graph/wire work behind timers (you measure an enqueue), while gunmetal does HAM, events, and wire serialization inline (you measure the work). The gunmetal client runs with gap:10 wire batching here, matching GUN's default ~1ms outbound drain.",
			'RTT is measured FIRST on a clean socket — after the put floods it would report queue-drain time, not round-trip time.',
			'Numbers vary with machine load — run on an idle machine and compare medians of several runs.'
		]
	},
	'gunjs-interop': {
		why: 'Wire compatibility is the difference between "inspired by GUN" and "is a GUN peer". One shared input, two engines — the unmodified GUN.js library and gunmetal wasm — through one Rust relay, is the parity spec\'s acceptance test running live.',
		snippets: [
			{
				label: 'GUN.js side',
				code: `const gun = Gun({ peers: ['ws://localhost:8765/gun'],
  localStorage: false, radisk: false, axe: false, multicast: false });
gun.get('doc').get('text').on(render);
gun.get('doc').get('text').put('hello from GUN.js');`
			},
			{
				label: 'gunmetal side',
				code: `const gun = WasmGun.withOptions(JSON.stringify({ localStorage: false }));
gun.connect('ws://localhost:8765/gun');
gun.on('doc', 'text', (json) => render(JSON.parse(json)));
gun.putText('doc', 'text', 'hello from gunmetal');`
			}
		],
		gotchas: [
			'GUN.js defaults localStorage to ON — stale local state replays into the mesh on boot unless you disable it for ephemeral clients.',
			'GUN.js only auto-detects window.WebSocket; under Node pass WebSocket explicitly in the constructor options.'
		]
	}
};

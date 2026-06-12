/**
 * The capabilities manifest — single source of truth for the gunmetal
 * catalog's information architecture (spec §3.2).
 *
 * Everything derives from this file: sidebar groups and ordering, the demo
 * grid and its filtering, per-demo related links, per-chapter embeds and
 * footers, per-module "demos using this" lists, prerender entries for all
 * dynamic routes, and the link-integrity e2e test. The learn↔demo↔reference
 * triangle is generated, never hand-maintained.
 */

export type CapabilityId =
	| 'core-graph'
	| 'reactivity'
	| 'collections'
	| 'documents'
	| 'sync'
	| 'conflict'
	| 'identity'
	| 'privacy'
	| 'permissions'
	| 'persistence'
	| 'mesh'
	| 'wire-compat';

export interface Capability {
	id: CapabilityId;
	title: string;
	/** One-line claim for the landing capability grid. */
	claim: string;
	/** Status badge when partial or relay-dependent. */
	badge?: string;
	/** The demo that owns this capability (landing grid links here). */
	demo: string;
}

export interface Demo {
	slug: string;
	title: string;
	/** The archetypal web pattern this demo embodies. */
	pattern: string;
	capability: CapabilityId;
	/** gunmetal modules exercised (reference-page names). */
	modules: string[];
	/** Learn chapter this demo embeds in (slug), if any. */
	chapter: string | null;
	/** Riley's 30-second wow — pinned first in the grid. */
	flagship?: boolean;
	/** Runs without a relay in a single frame (early learn chapters). */
	singleFrame?: boolean;
	/** Per-frame engine overrides (frameId → engine), e.g. gunjs-interop
	 * boots real GUN.js in frame "a" and gunmetal wasm in frame "b". */
	engines?: Record<string, 'gun' | 'gunmetal'>;
	/** Search aliases: API symbols, synonyms, symptoms. */
	keywords: string[];
}

export interface Chapter {
	/** 0-based position in the learn path. */
	num: number;
	slug: string;
	title: string;
	/** Short sidebar label. */
	label: string;
	/** Demo slugs embedded in this chapter (single-frame before sync). */
	embeds: string[];
	/** Reference modules this chapter links in its footer. */
	refs: string[];
	keywords: string[];
}

export interface ModuleRef {
	name: string;
	purpose: string;
	surface: 'both' | 'native-only' | 'wasm-only';
	keywords: string[];
}

// ── Capabilities ─────────────────────────────────────────────────────

export const capabilities: Capability[] = [
	{ id: 'core-graph', title: 'Core graph model', claim: 'Souls, nodes, links — put and get on a distributed graph', demo: 'graph-explorer' },
	{ id: 'reactivity', title: 'Reactivity', claim: 'Subscriptions fire on every change, local or remote', demo: 'graph-explorer' },
	{ id: 'collections', title: 'Collections & queries', claim: 'Sets with time-sortable ids, LEX range filters', demo: 'todo-list' },
	{ id: 'documents', title: 'Documents', claim: 'Deep paths and full-document assembly over linked nodes', demo: 'profile-tree' },
	{ id: 'sync', title: 'Realtime sync', claim: 'Live state shared across sessions through a relay', demo: 'shared-input' },
	{ id: 'conflict', title: 'Conflict resolution', claim: 'HAM CRDT merges concurrent edits deterministically', demo: 'conflict-lab' },
	{ id: 'identity', title: 'Identity', claim: 'Keypair users, password auth, signed data', demo: 'login' },
	{ id: 'privacy', title: 'Privacy', claim: 'End-to-end encryption and ECDH shared secrets', demo: 'private-notes' },
	{ id: 'permissions', title: 'Permissions', claim: 'Certificates delegate write access without a server', demo: 'doc-permissions' },
	{ id: 'persistence', title: 'Persistence', claim: 'RAD storage: IndexedDB in the browser, radata on the relay', demo: 'offline-first' },
	{ id: 'mesh', title: 'Mesh networking', claim: 'DAM routing: dedup, batching, ACK tracing, AXE subscriptions', badge: 'relay topology; no WebRTC', demo: 'wire-inspector' },
	{ id: 'wire-compat', title: 'GUN.js wire compatibility', claim: 'Gunmetal and GUN.js clients on the same mesh', demo: 'gunjs-interop' }
];

// ── Demos ────────────────────────────────────────────────────────────

export const demos: Demo[] = [
	{
		slug: 'gunjs-interop',
		title: 'GUN.js interop',
		pattern: 'Two clients, two engines, one shared input — the left iframe runs real GUN.js, the right runs gunmetal wasm',
		capability: 'wire-compat',
		modules: ['wire', 'mesh', 'relay'],
		chapter: 'gun-mesh',
		flagship: true,
		engines: { a: 'gun', b: 'gunmetal' },
		keywords: ['gun.js', 'interop', 'wire', 'compatibility', 'parity', 'flagship']
	},
	{
		slug: 'graph-explorer',
		title: 'Graph explorer',
		pattern: 'Object inspector / data editor — single session, no relay',
		capability: 'core-graph',
		modules: ['instance', 'graph', 'types', 'events'],
		chapter: 'the-graph',
		singleFrame: true,
		keywords: ['put', 'get', 'val', 'soul', 'node', 'link', 'on', 'subscribe', 'inspector']
	},
	{
		slug: 'shared-input',
		title: 'Shared input',
		pattern: 'A form field two sessions edit together',
		capability: 'sync',
		modules: ['instance', 'mesh', 'transport'],
		chapter: 'sync',
		keywords: ['realtime', 'sync', 'collaborative', 'input', 'peers', 'relay', 'put', 'on']
	},
	{
		slug: 'chat-room',
		title: 'Chat room',
		pattern: 'Messages in order, with history pagination',
		capability: 'collections',
		modules: ['uuid', 'lex', 'instance'],
		chapter: 'collections',
		keywords: ['chat', 'messages', 'set', 'uuid', 'ordering', 'lex', 'range', 'pagination', 'history', 'map']
	},
	{
		slug: 'todo-list',
		title: 'Todo list',
		pattern: 'A collaborative list: add, toggle, remove',
		capability: 'collections',
		modules: ['extended', 'instance', 'uuid'],
		chapter: 'collections',
		keywords: ['todo', 'list', 'set', 'unset', 'remove', 'delete', 'collection', 'map']
	},
	{
		slug: 'presence',
		title: 'Presence',
		pattern: '"Who\'s online" avatars with last-seen timestamps',
		capability: 'mesh',
		modules: ['mesh', 'extended'],
		chapter: 'sync',
		keywords: ['presence', 'online', 'heartbeat', 'bye', 'last seen', 'peers', 'join', 'leave']
	},
	{
		slug: 'conflict-lab',
		title: 'Conflict lab',
		pattern: 'Split-brain editor: disconnect, diverge, reconnect, converge',
		capability: 'conflict',
		modules: ['crdt', 'state'],
		chapter: 'conflict',
		keywords: ['conflict', 'crdt', 'ham', 'merge', 'converge', 'offline', 'split-brain', 'last write wins']
	},
	{
		slug: 'login',
		title: 'Login',
		pattern: 'Signup, login, session restore',
		capability: 'identity',
		modules: ['user', 'sea'],
		chapter: 'identity',
		keywords: ['login', 'signup', 'auth', 'user', 'create', 'authPair', 'session', 'keypair', 'password', 'signed']
	},
	{
		slug: 'private-notes',
		title: 'Private notes',
		pattern: 'Notes only their author can read',
		capability: 'privacy',
		modules: ['sea'],
		chapter: 'privacy',
		keywords: ['encrypt', 'decrypt', 'private', 'notes', 'work', 'aes', 'ciphertext']
	},
	{
		slug: 'secret-handshake',
		title: 'Secret handshake',
		pattern: 'Direct messages between two users',
		capability: 'privacy',
		modules: ['sea', 'user'],
		chapter: 'privacy',
		keywords: ['secret', 'dm', 'direct message', 'ecdh', 'shared secret', 'epub', 'epriv']
	},
	{
		slug: 'doc-permissions',
		title: 'Doc permissions',
		pattern: 'A shared doc whose owner grants edit access to a guest',
		capability: 'permissions',
		modules: ['cert', 'user'],
		chapter: 'permissions',
		keywords: ['certificate', 'cert', 'grant', 'permission', 'access', 'delegate', 'write', 'acl']
	},
	{
		slug: 'profile-tree',
		title: 'Profile tree',
		pattern: 'A nested profile editor with linked nodes and empty states',
		capability: 'documents',
		modules: ['extended'],
		chapter: 'documents',
		singleFrame: true,
		keywords: ['path', 'open', 'load', 'not', 'document', 'nested', 'tree', 'empty state', 'deep']
	},
	{
		slug: 'offline-first',
		title: 'Offline first',
		pattern: 'Disconnect from the relay, keep editing, reload — still there; reconnect — it syncs',
		capability: 'persistence',
		modules: ['rad', 'storage'],
		chapter: 'persistence',
		keywords: ['offline', 'persistence', 'indexeddb', 'rad', 'radisk', 'radata', 'reload', 'storage', 'local']
	},
	{
		slug: 'wire-inspector',
		title: 'Wire inspector',
		pattern: 'A live wire-protocol log beside a working app',
		capability: 'mesh',
		modules: ['wire', 'mesh', 'dup'],
		chapter: 'sync',
		keywords: ['wire', 'dam', 'debug', 'inspector', 'frames', 'ack', 'dedup', 'heartbeat', 'axe', 'not syncing', 'troubleshoot']
	}
];

// ── Learn chapters ───────────────────────────────────────────────────

export const chapters: Chapter[] = [
	{
		num: 0,
		slug: 'why-decentralized',
		title: "Why decentralized? What's a graph?",
		label: 'Why decentralized?',
		embeds: [],
		refs: [],
		keywords: ['intro', 'graph database', 'decentralized', 'gun.js', 'soul', 'beginner', 'start']
	},
	{
		num: 1,
		slug: 'the-graph',
		title: 'The graph — nodes, links, put/get',
		label: 'The graph',
		embeds: ['graph-explorer'],
		refs: ['instance', 'types', 'graph'],
		keywords: ['put', 'get', 'node', 'soul', 'link', 'graph']
	},
	{
		num: 2,
		slug: 'reactivity',
		title: 'Reactivity — subscriptions and chains',
		label: 'Reactivity',
		embeds: ['graph-explorer'],
		refs: ['instance', 'events'],
		keywords: ['on', 'once', 'off', 'subscribe', 'chain', 'reactive', 'callback']
	},
	{
		num: 3,
		slug: 'collections',
		title: 'Collections & queries — sets, ids, filters',
		label: 'Collections',
		embeds: ['todo-list', 'chat-room'],
		refs: ['uuid', 'lex'],
		keywords: ['set', 'uuid', 'map', 'lex', 'collection', 'list', 'query', 'filter', 'unset']
	},
	{
		num: 4,
		slug: 'documents',
		title: 'Documents — deep paths, open and load',
		label: 'Documents',
		embeds: ['profile-tree'],
		refs: ['extended'],
		keywords: ['path', 'open', 'load', 'document', 'nested', 'not']
	},
	{
		num: 5,
		slug: 'sync',
		title: 'Sync — peers and the relay',
		label: 'Sync',
		embeds: ['shared-input', 'presence'],
		refs: ['mesh', 'transport', 'relay'],
		keywords: ['peer', 'relay', 'sync', 'websocket', 'server', 'message', 'wire']
	},
	{
		num: 6,
		slug: 'conflict',
		title: "Conflict — HAM, and why last-write-wins isn't enough",
		label: 'Conflict',
		embeds: ['conflict-lab'],
		refs: ['crdt', 'state'],
		keywords: ['ham', 'crdt', 'conflict', 'merge', 'converge', 'state', 'timestamp']
	},
	{
		num: 7,
		slug: 'persistence',
		title: 'Persistence — your data survives reload',
		label: 'Persistence',
		embeds: ['offline-first'],
		refs: ['rad', 'storage'],
		keywords: ['persistence', 'rad', 'indexeddb', 'radata', 'storage', 'offline', 'reload']
	},
	{
		num: 8,
		slug: 'identity',
		title: 'Identity — keypairs and users',
		label: 'Identity',
		embeds: ['login'],
		refs: ['user', 'sea'],
		keywords: ['sea', 'user', 'auth', 'identity', 'keypair', 'sign', 'verify', 'login']
	},
	{
		num: 9,
		slug: 'privacy',
		title: 'Privacy — encryption and shared secrets',
		label: 'Privacy',
		embeds: ['private-notes'],
		refs: ['sea'],
		keywords: ['encrypt', 'decrypt', 'secret', 'privacy', 'ecdh', 'work']
	},
	{
		num: 10,
		slug: 'permissions',
		title: 'Permissions — certificates',
		label: 'Permissions',
		embeds: ['doc-permissions'],
		refs: ['cert'],
		keywords: ['certificate', 'cert', 'permission', 'grant', 'delegate', 'access']
	},
	{
		num: 11,
		slug: 'gun-mesh',
		title: 'Epilogue: joining the GUN mesh',
		label: 'The GUN mesh',
		embeds: ['gunjs-interop'],
		refs: ['wire', 'mesh', 'relay'],
		keywords: ['gun.js', 'interop', 'wire', 'compatibility', 'mesh', 'graduation']
	}
];

// ── Reference modules ────────────────────────────────────────────────

export const modules: ModuleRef[] = [
	{ name: 'instance', purpose: 'Gun + GunChain — the main API surface, GunOptions', surface: 'both', keywords: ['gun', 'chain', 'options', 'put', 'get', 'val', 'on', 'once', 'map', 'set', 'GunOptions', 'peers', 'localStorage', 'radisk'] },
	{ name: 'types', purpose: 'GunValue, Node, Soul — the core data model', surface: 'both', keywords: ['GunValue', 'Node', 'Soul', 'types', 'is_node', 'soul'] },
	{ name: 'graph', purpose: 'In-memory graph store with LRU eviction', surface: 'both', keywords: ['graph', 'lru', 'eviction', 'memory', 'pinning'] },
	{ name: 'crdt', purpose: 'HAM conflict resolution', surface: 'both', keywords: ['ham', 'crdt', 'conflict', 'merge', 'drift'] },
	{ name: 'state', purpose: 'State vectors and timestamps', surface: 'both', keywords: ['state', 'timestamp', 'vector', 'now'] },
	{ name: 'sea', purpose: 'ECDSA signing, AES-GCM encryption, PBKDF2, ECDH secrets', surface: 'both', keywords: ['sea', 'pair', 'sign', 'verify', 'encrypt', 'decrypt', 'work', 'secret', 'crypto'] },
	{ name: 'user', purpose: 'Decentralized auth and SignedChain', surface: 'both', keywords: ['user', 'create', 'auth', 'authPair', 'leave', 'signed', 'alias'] },
	{ name: 'cert', purpose: 'Certificates for delegated write permissions', surface: 'both', keywords: ['certificate', 'cert', 'grants_access', 'revocation', 'tombstone', 'delegate'] },
	{ name: 'sync', purpose: 'Peer replication and GET handling', surface: 'both', keywords: ['sync', 'replication', 'flush', 'SyncPair'] },
	{ name: 'mesh', purpose: 'DAM mesh routing: dedup, batching, ACK tracing, AXE', surface: 'both', keywords: ['mesh', 'dam', 'hear', 'say', 'hi', 'bye', 'axe', 'mob', 'heartbeat', 'pid', 'subscription'] },
	{ name: 'relay', purpose: 'GUN.js-compatible WebSocket relay server', surface: 'native-only', keywords: ['relay', 'server', 'websocket', 'port', '8765', 'health', 'tls', 'radata', 'gunmetal-relay'] },
	{ name: 'extended', purpose: 'gun/lib/* plugins: path, open, load, not, unset, then, later, bye', surface: 'both', keywords: ['path', 'open', 'load', 'not', 'unset', 'then', 'promise', 'later', 'bye', 'extended'] },
	{ name: 'storage', purpose: 'Sync/async persistence adapters and BatchWriter', surface: 'both', keywords: ['storage', 'adapter', 'BatchWriter', 'memory', 'indexeddb', 'engine'] },
	{ name: 'rad', purpose: 'RAD radix storage engine: chunked, batched persistence', surface: 'both', keywords: ['rad', 'radix', 'radisk', 'chunk', 'radata', 'fs_store', 'idb_store'] },
	{ name: 'transport', purpose: 'WebSocket transport, reconnection, peer tracking', surface: 'both', keywords: ['transport', 'websocket', 'reconnect', 'backoff', 'peers'] },
	{ name: 'wire', purpose: 'JSON wire protocol: PUT/GET/ACK messages, DAM fields', surface: 'both', keywords: ['wire', 'WireMessage', 'put', 'get', 'ack', 'dam', 'hash', 'seen_by', 'protocol'] },
	{ name: 'events', purpose: 'Pub-sub event bus', surface: 'both', keywords: ['events', 'EventBus', 'emit', 'listener'] },
	{ name: 'uuid', purpose: 'Time-sortable UUID generation for collections', surface: 'both', keywords: ['uuid', 'id', 'time-sortable', 'random_message_id'] },
	{ name: 'lex', purpose: 'LEX queries: exact, prefix, range', surface: 'both', keywords: ['lex', 'query', 'prefix', 'range', 'gte', 'lte', 'filter'] },
	{ name: 'dup', purpose: 'Message dedup table with via-tracing', surface: 'both', keywords: ['dup', 'dedup', 'via', 'track', 'ttl'] },
	{ name: 'concurrency', purpose: 'Platform-gated shared state (Arc/Rc)', surface: 'both', keywords: ['concurrency', 'arc', 'rc', 'mutex', 'MaybeSend'] },
	{ name: 'runtime', purpose: 'Cross-platform spawn and sleep', surface: 'both', keywords: ['runtime', 'spawn', 'sleep', 'tokio', 'wasm'] }
];

// ── Derived lookups ──────────────────────────────────────────────────

export const demoBySlug = new Map(demos.map((d) => [d.slug, d]));
export const chapterBySlug = new Map(chapters.map((c) => [c.slug, c]));
export const moduleByName = new Map(modules.map((m) => [m.name, m]));
export const capabilityById = new Map(capabilities.map((c) => [c.id, c]));

/** Demos exercising a module (reference pages' "demos using this"). */
export function demosForModule(name: string): Demo[] {
	return demos.filter((d) => d.modules.includes(name));
}

/** Demos grouped for the catalog grid: flagship pinned first. */
export function demosForGrid(): Demo[] {
	return [...demos].sort((a, b) => Number(b.flagship ?? false) - Number(a.flagship ?? false));
}

/** Chapters that embed a demo (demo pages' "learn this concept"). */
export function chaptersForDemo(slug: string): Chapter[] {
	return chapters.filter((c) => c.embeds.includes(slug));
}

export const orderedChapters = [...chapters].sort((a, b) => a.num - b.num);

/** Every shell URL the manifest implies — drives prerender entries and
 * the link-integrity test. */
export function allRoutes(): string[] {
	return [
		'/',
		'/maproom',
		'/crewchief',
		'/gunmetal',
		'/gunmetal/quickstart',
		'/gunmetal/troubleshooting',
		'/gunmetal/glossary',
		'/gunmetal/learn',
		...orderedChapters.map((c) => `/gunmetal/learn/${c.slug}`),
		'/gunmetal/demos',
		...demos.map((d) => `/gunmetal/demos/${d.slug}`),
		...demos.map((d) => `/gunmetal/demos/${d.slug}/client`),
		'/gunmetal/reference',
		...modules.map((m) => `/gunmetal/reference/${m.name}`)
	];
}

/**
 * Client-page boot library: wasm init, relay connection, and the
 * readiness contract (spec §5.5).
 *
 * Readiness: `document.body.dataset.ready = 'true'` and the
 * `window.__gmReady` promise resolve ONLY after wasm is instantiated,
 * the relay WebSocket is open, the DAM handshake is acked (peerPid set),
 * and the demo's subscriptions are registered. Every Playwright spec
 * waits on `body[data-ready="true"]` before interacting. A relay that
 * can't be reached produces `data-ready="degraded"` plus visible run
 * instructions — never a hung spinner.
 */

export interface ClientParams {
	room: string;
	frameId: string;
	relay: string;
	engine: 'gunmetal' | 'gun';
	singleFrame: boolean;
}

export type WasmGunInstance = InstanceType<
	typeof import('$lib/wasm/gunmetal.js').WasmGun
>;

export function parseClientParams(loc: Location): ClientParams {
	const q = new URLSearchParams(loc.search);
	return {
		room: q.get('room') ?? 'lobby',
		frameId: q.get('frameId') ?? 'solo',
		relay: q.get('relay') ?? `ws://${loc.hostname}:8765/gun`,
		engine: q.get('engine') === 'gun' ? 'gun' : 'gunmetal',
		singleFrame: q.get('frames') === '1'
	};
}

declare global {
	interface Window {
		__gmReady?: Promise<void>;
		__gmReadyResolve?: () => void;
	}
}

export function installReadyPromise(): void {
	window.__gmReady = new Promise<void>((resolve) => {
		window.__gmReadyResolve = resolve;
	});
}

export function markReady(): void {
	document.body.dataset.ready = 'true';
	window.__gmReadyResolve?.();
}

export function markDegraded(): void {
	document.body.dataset.ready = 'degraded';
}

/** Boot the gunmetal wasm engine; connect to the relay unless this is a
 * single-frame (relay-less) variant. Throws on handshake timeout. */
export async function bootGunmetal(params: ClientParams): Promise<WasmGunInstance> {
	const wasm = await import('$lib/wasm/gunmetal.js');
	const wasmUrl = (await import('$lib/wasm/gunmetal_bg.wasm?url')).default;
	await wasm.default({ module_or_path: wasmUrl });

	// Demo clients are ephemeral: storage stays off until the
	// persistence demos wire it explicitly (with per-frame namespacing).
	const gun = wasm.WasmGun.withOptions(
		JSON.stringify({ localStorage: false, radisk: false, axe: false })
	);

	if (!params.singleFrame) {
		gun.connect(params.relay);
		await waitForHandshake(gun, params.relay, 10_000);
	}
	return gun;
}

/** The handshake-acked signal: the relay's pid learned via DAM `?`. */
export async function waitForHandshake(
	gun: WasmGunInstance,
	url: string,
	timeoutMs: number
): Promise<void> {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		if (gun.peerPid(url)) return;
		await new Promise((resolve) => setTimeout(resolve, 50));
	}
	throw new Error(`relay handshake timed out: ${url}`);
}

export function relayRunHint(relay: string): string {
	return `Relay unreachable at ${relay} — run it locally:\ncargo run -p gunmetal --features relay --bin gunmetal-relay -- --port 8765`;
}

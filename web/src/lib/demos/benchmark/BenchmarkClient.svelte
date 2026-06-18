<script lang="ts">
	// A/B engine benchmark (spec gunmetal-bench.md tier 2): this frame
	// runs IDENTICAL workloads on whichever engine ?engine= selects and
	// renders its own results — the demo page shows the two engines side
	// by side. A measurement instrument, not a regression gate.
	import { onMount } from 'svelte';
	import {
		bootGunmetal,
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		relayRunHint,
		wasmModule,
		type ClientParams,
		type WasmGunInstance
	} from '$lib/gun/client';

	let { slug = 'benchmark' }: { slug?: string } = $props();

	interface Row {
		label: string;
		ms: number;
		/** Sample standard deviation across passes (shown as ±σ). */
		spread?: number;
		ops?: number;
	}

	let engine = $state<'gun' | 'gunmetal' | null>(null);
	const PASSES = 5;

	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let running = $state(false);
	let done = $state(false);
	let pass = $state(0);
	let rows = $state<Row[]>([]);
	let hint = $state('');

	// Per-label samples across passes; rows render the MEDIAN.
	const samples = new Map<string, { ms: number[]; count: number | null }>();
	const sampleOrder: string[] = [];

	let params: ClientParams;
	let gmGun: WasmGunInstance | undefined;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let gmSea: any;
	// gunmetal put-ack correlation via the wire tap: out-put mid → sentAt.
	const gmPendingAcks = new Map<string, number>();
	const gmAckRtts: number[] = [];
	// Wire-quiet detection: fixed sleeps don't drain a flood's backlog —
	// RTT measured over residue reads seconds, not milliseconds.
	let gmLastWireAt = 0;
	// Subscriptions register ONCE — re-registering per pass would make
	// later passes pay for every earlier pass's dead listener.
	let gmSubFires = 0;
	let gmSubscribed = false;
	let gunSubFires = 0;
	let gunSubscribed = false;
	let passSeq = 0; // distinct values per pass so every put is a real change

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let gunJs: any;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let gunSEA: any;

	let base = '';

	async function loadScript(src: string): Promise<void> {
		await new Promise<void>((resolve, reject) => {
			const script = document.createElement('script');
			script.src = src;
			script.onload = () => resolve();
			script.onerror = () => reject(new Error(`failed to load ${src}`));
			document.head.appendChild(script);
		});
	}

	onMount(async () => {
		installReadyPromise();
		params = parseClientParams(window.location);
		engine = params.engine;
		base = `bench/${params.room}/${params.frameId}`;

		try {
			if (engine === 'gunmetal') {
				// gap:10 enables wire batching — GUN.js batches its outbound
				// by default (~1ms drain), so unbatched gunmetal would
				// benchmark a per-put network flood against an enqueue.
				gmGun = await bootGunmetal(params, { gap: 10 });
				gmSea = new (await wasmModule()).WasmSEA();
				gmGun.onWire((dir: string, _peer: string, raw: string) => {
					gmLastWireAt = performance.now();
					try {
						const frames = raw.startsWith('[') ? JSON.parse(raw) : [JSON.parse(raw)];
						for (const frame of frames) {
							if (dir === 'out' && frame?.put?.[`${base}/ack`] && frame['#']) {
								gmPendingAcks.set(frame['#'], performance.now());
							} else if (dir === 'in' && typeof frame?.['@'] === 'string') {
								const sentAt = gmPendingAcks.get(frame['@']);
								if (sentAt !== undefined) {
									gmAckRtts.push(performance.now() - sentAt);
									gmPendingAcks.delete(frame['@']);
								}
							}
						}
					} catch {
						/* heartbeats etc. */
					}
				});
			} else {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const w = window as any;
				if (!w.Gun) await loadScript('/vendor/gun.js');
				if (!w.Gun.SEA) await loadScript('/vendor/sea.js');
				gunSEA = w.Gun.SEA;
				gunJs = w.Gun({
					peers: [params.relay],
					localStorage: false,
					radisk: false,
					axe: false,
					multicast: false
				});
				// readiness = relay acks a probe put (same as the interop demo)
				await new Promise<void>((resolve, reject) => {
					const timer = setTimeout(() => reject(new Error('relay timeout')), 10_000);
					gunJs.get(`${base}/probe`).put({ ping: 1 }, (ack: { err?: string }) => {
						clearTimeout(timer);
						ack.err ? reject(new Error(ack.err)) : resolve();
					});
				});
			}
			status = 'ready';
			markReady();
		} catch (cause) {
			hint = relayRunHint(params.relay);
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});

	const median = (xs: number[]) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)] ?? 0;

	function stddev(xs: number[]): number | undefined {
		if (xs.length < 2) return undefined;
		const mean = xs.reduce((a, b) => a + b, 0) / xs.length;
		return Math.sqrt(xs.reduce((a, b) => a + (b - mean) ** 2, 0) / (xs.length - 1));
	}

	function record(label: string, ms: number, count: number | null) {
		if (!samples.has(label)) {
			samples.set(label, { ms: [], count });
			sampleOrder.push(label);
		}
		samples.get(label)!.ms.push(ms);
		// re-render rows as median ± σ of completed samples
		rows = sampleOrder.map((rowLabel) => {
			const sample = samples.get(rowLabel)!;
			const mid = median(sample.ms);
			return {
				label: rowLabel,
				ms: mid,
				spread: stddev(sample.ms),
				ops: sample.count ? sample.count / (mid / 1000) : undefined
			};
		});
	}

	async function measure(label: string, count: number | null, work: () => Promise<void> | void) {
		const start = performance.now();
		await work();
		record(label, performance.now() - start, count);
		// let the UI breathe between workloads
		await new Promise((resolve) => setTimeout(resolve, 50));
	}

	/** Wait until no wire frames for `windowMs` (capped) — the only
	 * honest "drained" signal after a put flood. */
	async function gmQuiet(windowMs = 400, capMs = 10_000) {
		const start = performance.now();
		while (performance.now() - start < capMs) {
			if (performance.now() - gmLastWireAt > windowMs) return;
			await new Promise((r) => setTimeout(r, 50));
		}
	}

	async function runGunmetal() {
		const gun = gmGun!;
		const p = passSeq;
		// RTT first, and only once the relay is RESPONSIVE again: a put
		// flood saturates the relay's inbound for a couple of seconds (its
		// slow-consumer policy drops the flood's acks, so the wire looks
		// quiet) — RTT measured during recovery reports the backlog, not
		// the round trip. Canary puts gate until an ack comes back fast.
		await gmQuiet();
		if (gun.isConnected(params.relay)) {
			const gateDeadline = Date.now() + 15_000;
			while (Date.now() < gateDeadline) {
				gmAckRtts.length = 0;
				gun.putText(`${base}/ack`, 'v', `canary-${p}-${Date.now()}`);
				gun.flushMesh();
				const canaryWait = Date.now() + 500;
				while (gmAckRtts.length === 0 && Date.now() < canaryWait)
					await new Promise((r) => setTimeout(r, 20));
				if (gmAckRtts.length > 0 && gmAckRtts[0] < 100) break;
			}
			gmAckRtts.length = 0;
			for (let i = 0; i < 20; i++) {
				gun.putText(`${base}/ack`, 'v', `rtt-${p}-${i}`);
				gun.flushMesh();
				await new Promise((r) => setTimeout(r, 60));
			}
			const deadline = Date.now() + 5000;
			while (gmAckRtts.length < 15 && Date.now() < deadline)
				await new Promise((r) => setTimeout(r, 50));
			// only record RTT when acks were actually captured — median([]) → 0
			// would otherwise report a bogus "0 ms" round-trip.
			if (gmAckRtts.length) record('put→ack RTT (median ms)', median(gmAckRtts), null);
		}
		await measure('local puts ×3000', 3000, () => {
			for (let i = 0; i < 3000; i++) gun.putText(`${base}/local`, `k${i}`, `v${p}-${i}`);
		});
		gun.flushMesh();
		await gmQuiet(); // drain before next workload
		await measure('subscription fires ×1000', 1000, async () => {
			if (!gmSubscribed) {
				gmSubscribed = true;
				gun.on(`${base}/subs`, 'v', () => gmSubFires++);
			}
			const before = gmSubFires;
			for (let i = 0; i < 1000; i++) gun.putText(`${base}/subs`, 'v', `value-${p}-${i}`);
			const deadline = Date.now() + 5000;
			while (gmSubFires - before < 1000 && Date.now() < deadline)
				await new Promise((r) => setTimeout(r, 10));
		});
		gun.flushMesh();
		await gmQuiet();
		const pair = JSON.parse(gmSea.pair());
		await measure('SEA pair', null, () => void gmSea.pair());
		let signed = '';
		await measure('SEA sign ×50', 50, () => {
			for (let i = 0; i < 50; i++) signed = gmSea.sign(JSON.stringify(`msg-${i}`), pair.priv, pair.pub);
		});
		await measure('SEA verify ×50', 50, () => {
			for (let i = 0; i < 50; i++) gmSea.verify(signed, pair.pub);
		});
		let ct = '';
		await measure('SEA encrypt ×200', 200, () => {
			for (let i = 0; i < 200; i++) ct = gmSea.encrypt(JSON.stringify(`note-${i}`), 'bench-key');
		});
		await measure('SEA decrypt ×200', 200, () => {
			for (let i = 0; i < 200; i++) gmSea.decrypt(ct, 'bench-key');
		});
		await measure('SEA work (PBKDF2) ×3', 3, () => {
			for (let i = 0; i < 3; i++) gmSea.work('a passphrase', 'fixed-salt');
		});
	}

	async function runGunJs() {
		const p = passSeq;
		// RTT first, gated on relay responsiveness (matching gunmetal):
		// measured during post-flood recovery it reports the backlog.
		const canaryDeadline = Date.now() + 15_000;
		while (Date.now() < canaryDeadline) {
			const start = performance.now();
			const rtt = await new Promise<number>((resolve) => {
				gunJs.get(`${base}/ack`).get('v').put(`canary-${p}-${Date.now()}`, () =>
					resolve(performance.now() - start)
				);
				setTimeout(() => resolve(Infinity), 500);
			});
			if (rtt < 100) break;
		}
		const rtts: number[] = [];
		for (let i = 0; i < 20; i++) {
			const start = performance.now();
			await new Promise<void>((resolve) => {
				gunJs.get(`${base}/ack`).get('v').put(`rtt-${p}-${i}`, () => {
					rtts.push(performance.now() - start);
					resolve();
				});
				setTimeout(resolve, 2000); // never hang a workload on a lost ack
			});
			await new Promise((r) => setTimeout(r, 60));
		}
		// only record RTT when acks were actually captured (see gunmetal path)
		if (rtts.length) record('put→ack RTT (median ms)', median(rtts), null);

		await measure('local puts ×3000', 3000, () => {
			for (let i = 0; i < 3000; i++) gunJs.get(`${base}/local`).get(`k${i}`).put(`v${p}-${i}`);
		});
		await new Promise((r) => setTimeout(r, 1000)); // drain before next workload
		await measure('subscription fires ×1000', 1000, async () => {
			if (!gunSubscribed) {
				gunSubscribed = true;
				gunJs.get(`${base}/subs`).get('v').on(() => gunSubFires++);
			}
			const before = gunSubFires;
			for (let i = 0; i < 1000; i++) gunJs.get(`${base}/subs`).get('v').put(`value-${p}-${i}`);
			const deadline = Date.now() + 5000;
			while (gunSubFires - before < 1000 && Date.now() < deadline)
				await new Promise((r) => setTimeout(r, 10));
		});
		await new Promise((r) => setTimeout(r, 1000));
		const pair = await gunSEA.pair();
		await measure('SEA pair', null, async () => void (await gunSEA.pair()));
		let signed: unknown;
		await measure('SEA sign ×50', 50, async () => {
			for (let i = 0; i < 50; i++) signed = await gunSEA.sign(`msg-${i}`, pair);
		});
		await measure('SEA verify ×50', 50, async () => {
			for (let i = 0; i < 50; i++) await gunSEA.verify(signed, pair.pub);
		});
		let ct: unknown;
		await measure('SEA encrypt ×200', 200, async () => {
			for (let i = 0; i < 200; i++) ct = await gunSEA.encrypt(`note-${i}`, 'bench-key');
		});
		await measure('SEA decrypt ×200', 200, async () => {
			for (let i = 0; i < 200; i++) await gunSEA.decrypt(ct, 'bench-key');
		});
		await measure('SEA work (PBKDF2) ×3', 3, async () => {
			for (let i = 0; i < 3; i++) await gunSEA.work('a passphrase', 'fixed-salt');
		});
	}

	async function run() {
		if (running) return;
		running = true;
		done = false;
		rows = [];
		samples.clear();
		sampleOrder.length = 0;
		try {
			for (pass = 1; pass <= PASSES; pass++) {
				passSeq++;
				if (engine === 'gunmetal') await runGunmetal();
				else await runGunJs();
				await new Promise((r) => setTimeout(r, 200)); // settle between passes
			}
		} finally {
			running = false;
			done = true;
		}
	}

	const fmt = (n: number) =>
		n >= 100 ? Math.round(n).toLocaleString() : n.toFixed(1);
</script>

<div class="flex min-h-screen flex-col gap-3 bg-background p-4 text-foreground" data-bench-done={done}>
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting {engine ?? '…'}…</p>
	{:else if status === 'degraded'}
		<div class="rounded-lg border border-destructive/50 p-4 text-sm" data-testid="client-degraded">
			<p class="font-medium">Relay unreachable</p>
			<pre class="mt-2 whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		</div>
	{:else}
		<div class="flex items-center justify-between">
			<p class="text-xs uppercase tracking-wide text-muted-foreground">
				Engine: <strong>{engine === 'gun' ? 'GUN.js' : 'gunmetal (wasm)'}</strong>
			</p>
			<button
				class="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground disabled:opacity-50"
				onclick={run}
				disabled={running}
				data-testid="bench-run"
			>
				{running ? `Running… pass ${pass}/${PASSES}` : done ? 'Run again' : 'Run benchmarks'}
			</button>
		</div>
		{#if rows.length > 0}
			<p class="text-[10px] text-muted-foreground">
				median of {PASSES} runs · ±σ = sample standard deviation across runs
			</p>
		{/if}
		<table class="w-full text-xs" data-testid="bench-results">
			<tbody>
				{#each rows as row (row.label)}
					<tr class="border-b border-border/50" data-testid="bench-row">
						<td class="py-1 pr-2">{row.label}</td>
						<td class="py-1 text-right font-mono">
							{fmt(row.ms)} ms{#if row.spread !== undefined}<span class="text-muted-foreground"> ±{fmt(row.spread)}</span>{/if}
						</td>
						<td class="py-1 pl-2 text-right font-mono text-muted-foreground">
							{row.ops ? `${fmt(row.ops)} ops/s` : ''}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
		{#if engine === 'gunmetal' && done}
			<p class="text-[10px] text-muted-foreground">
				wasm build: opt-level=z (size-optimized) — CPU numbers are conservative.
			</p>
		{/if}
	{/if}
</div>

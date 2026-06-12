<script lang="ts">
	// Devon's debugging companion: a working shared input with EVERY wire
	// frame this client sends or receives printed beside it — DAM
	// handshakes, puts, gets, acks, heartbeats, exactly as they cross the
	// WebSocket (fed by WasmGun.onWire).
	import { onMount } from 'svelte';
	import {
		bootGunmetal,
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		relayRunHint,
		wasmModule,
		type WasmGunInstance
	} from '$lib/gun/client';

	let { slug = 'wire-inspector' }: { slug?: string } = $props();

	interface WireLine {
		id: number;
		dir: 'in' | 'out';
		kind: string;
		raw: string;
	}

	let value = $state('');
	let log = $state<WireLine[]>([]);
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');

	let gun: WasmGunInstance | undefined;
	let soul = '';
	let nextId = 0;

	function classify(raw: string): string {
		if (raw === '[]') return 'heartbeat';
		try {
			const first = raw.startsWith('[') ? JSON.parse(raw)[0] : JSON.parse(raw);
			if (!first || typeof first !== 'object') return 'other';
			if (first.dam) return `dam:${first.dam}`;
			if (first.put && first['@']) return 'ack+put';
			if (first.put) return 'put';
			if (first.get) return 'get';
			if (first['@']) return 'ack';
			return 'other';
		} catch {
			return 'other';
		}
	}

	const kindColor: Record<string, string> = {
		heartbeat: 'text-muted-foreground/60',
		put: 'text-green-600 dark:text-green-400',
		'ack+put': 'text-emerald-600 dark:text-emerald-400',
		get: 'text-blue-600 dark:text-blue-400',
		ack: 'text-amber-600 dark:text-amber-400'
	};

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		soul = `demo/${slug}/${params.room}`;

		try {
			// Pre-init the module so onWire is registered BEFORE connect —
			// the handshake itself must appear in the log.
			await wasmModule();
			const wasm = await import('$lib/wasm/gunmetal.js');
			const wasmUrl = (await import('$lib/wasm/gunmetal_bg.wasm?url')).default;
			await wasm.default({ module_or_path: wasmUrl });
			gun = wasm.WasmGun.withOptions(
				JSON.stringify({ localStorage: false, radisk: false, axe: false })
			);
			gun.onWire((dir: 'in' | 'out', _peer: string, raw: string) => {
				log = [...log, { id: nextId++, dir, kind: classify(raw), raw }].slice(-60);
			});
			gun.connect(params.relay);
			const start = Date.now();
			while (Date.now() - start < 10_000 && !gun.peerPid(params.relay)) {
				await new Promise((resolve) => setTimeout(resolve, 50));
			}
			if (!gun.peerPid(params.relay)) throw new Error('handshake timeout');

			gun.on(soul, 'text', (json: string) => {
				const incoming = JSON.parse(json);
				if (typeof incoming === 'string' && incoming !== value) value = incoming;
			});
			gun.fetchSoul(soul);

			status = 'ready';
			markReady();
		} catch (cause) {
			hint = relayRunHint(params.relay);
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});

	function handleInput() {
		gun?.putText(soul, 'text', value);
	}
</script>

<div class="flex min-h-screen flex-col gap-3 bg-background p-4 text-foreground">
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting gunmetal…</p>
	{:else if status === 'degraded'}
		<div class="rounded-lg border border-destructive/50 p-4 text-sm" data-testid="client-degraded">
			<p class="font-medium">Relay unreachable</p>
			<pre class="mt-2 whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		</div>
	{:else}
		<div>
			<p class="mb-1 text-xs uppercase tracking-wide text-muted-foreground">
				Session {frameId} — the app
			</p>
			<input
				data-testid="shared-input"
				class="w-full rounded-md border bg-background px-3 py-2 text-sm"
				bind:value
				oninput={handleInput}
				placeholder="Type — then read the wire…"
			/>
		</div>
		<div class="min-h-0 flex-1">
			<p class="mb-1 text-xs uppercase tracking-wide text-muted-foreground">The wire</p>
			<ul class="h-48 space-y-0.5 overflow-y-auto font-mono text-[10px]" data-testid="wire-log">
				{#each log as line (line.id)}
					<li class="flex gap-1 {kindColor[line.kind] ?? ''}" data-dir={line.dir} data-kind={line.kind}>
						<span class="shrink-0 font-semibold">{line.dir === 'in' ? '◀' : '▶'} {line.kind}</span>
						<span class="truncate text-muted-foreground">{line.raw}</span>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>

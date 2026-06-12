<script lang="ts">
	// The same shared input, driven by REAL GUN.js (vendored from the gun
	// submodule, served at /vendor/gun.js). Readiness follows the §5.5
	// contract; "handshake acked" here is the relay acknowledging a probe
	// PUT — the same signal the native interop test relies on.
	import { onMount } from 'svelte';
	import {
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		relayRunHint
	} from '$lib/gun/client';
	import { sendToParent } from '$lib/gun/frame-protocol';

	let { slug = 'gunjs-interop' }: { slug?: string } = $props();

	let value = $state('');
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');

	// gun.js has no types; its chain API is dynamic.
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let chain: any;

	async function loadGunJs(): Promise<any> {
		const w = window as any;
		if (w.Gun) return w.Gun;
		await new Promise<void>((resolve, reject) => {
			const script = document.createElement('script');
			script.src = '/vendor/gun.js';
			script.onload = () => resolve();
			script.onerror = () => reject(new Error('failed to load /vendor/gun.js'));
			document.head.appendChild(script);
		});
		return w.Gun;
	}

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		const soul = `demo/${slug}/${params.room}`;

		try {
			const Gun = await loadGunJs();
			const gun = Gun({
				peers: [params.relay],
				localStorage: false,
				radisk: false,
				axe: false,
				multicast: false
			});

			chain = gun.get(soul).get('text');
			chain.on((incoming: unknown) => {
				if (typeof incoming === 'string' && incoming !== value) {
					value = incoming;
				}
			});

			// Probe write: the relay's ack proves the wire round-trip.
			await new Promise<void>((resolve, reject) => {
				const timer = setTimeout(
					() => reject(new Error(`relay handshake timed out: ${params.relay}`)),
					10_000
				);
				gun.get(`${soul}/probe`).put({ ping: frameId }, (ack: { err?: string }) => {
					clearTimeout(timer);
					if (ack.err) reject(new Error(ack.err));
					else resolve();
				});
			});

			status = 'ready';
			markReady();
			sendToParent({ type: 'gm:status', frameId, status: 'ready' });
		} catch (cause) {
			hint = relayRunHint(params.relay);
			status = 'degraded';
			markDegraded();
			sendToParent({ type: 'gm:status', frameId, status: 'degraded' });
			console.error(cause);
		}
	});

	function handleInput() {
		chain?.put(value);
	}
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-3 bg-background p-6 text-foreground">
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting GUN.js…</p>
	{:else if status === 'degraded'}
		<div class="max-w-sm rounded-lg border border-destructive/50 p-4 text-sm" data-testid="client-degraded">
			<p class="font-medium">Relay unreachable</p>
			<pre class="mt-2 whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		</div>
	{:else}
		<label class="text-xs uppercase tracking-wide text-muted-foreground" for="shared">
			Session {frameId} — GUN.js — type here
		</label>
		<input
			id="shared"
			data-testid="shared-input"
			class="w-full max-w-sm rounded-md border bg-background px-3 py-2 text-base shadow-sm focus:outline-2"
			bind:value
			oninput={handleInput}
			placeholder="Synced through the relay…"
		/>
	{/if}
</div>

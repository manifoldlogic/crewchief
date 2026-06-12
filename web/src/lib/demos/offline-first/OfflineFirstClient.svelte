<script lang="ts">
	// Offline-first: IndexedDB persistence namespaced PER SESSION
	// (gunmetal-<room>-<frameId> — same-origin iframes share IndexedDB,
	// so distinct names are what keep these sessions honest), plus
	// relay-disconnect controls. "Offline" here means disconnected from
	// the relay — the page itself still loads (spec §4 note).
	import { onMount } from 'svelte';
	import {
		bootGunmetal,
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		relayRunHint,
		waitForHandshake,
		type WasmGunInstance
	} from '$lib/gun/client';
	import { onParentMessage, sendToParent } from '$lib/gun/frame-protocol';

	let { slug = 'offline-first' }: { slug?: string } = $props();

	let value = $state('');
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let relayState = $state<'connected' | 'disconnected'>('disconnected');
	let hint = $state('');
	let frameId = $state('');

	let gun: WasmGunInstance | undefined;
	let soul = '';
	let relay = '';
	// Re-announce on reconnect ONLY if edited while offline — otherwise a
	// session that idled offline would re-put its stale (possibly empty)
	// value with a fresher HAM state and beat real edits.
	let dirtyOffline = false;

	function subscribe() {
		gun?.on(soul, 'text', (json: string) => {
			const incoming = JSON.parse(json);
			if (typeof incoming === 'string' && incoming !== value) {
				value = incoming;
			}
		});
	}

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		relay = params.relay;
		soul = `demo/${slug}/${params.room}`;

		try {
			// Boot WITHOUT connecting; wire per-session persistence and
			// hydrate local data first — the app works before (and
			// without) the network.
			gun = await bootGunmetal({ ...params, singleFrame: true });
			subscribe();
			await gun.enablePersistence(`gunmetal-${params.room}-${frameId}`);
			const existing = gun.get(soul, 'text') as string | null;
			if (existing) {
				const parsed = JSON.parse(existing);
				if (typeof parsed === 'string') value = parsed;
			}

			// Then go online (unless the relay is unreachable — local
			// data still shows either way).
			try {
				gun.connect(relay);
				await waitForHandshake(gun, relay, 5_000);
				gun.fetchSoul(soul);
				relayState = 'connected';
			} catch {
				relayState = 'disconnected';
				hint = relayRunHint(relay);
			}

			onParentMessage(async (msg) => {
				if (!gun) return;
				if (msg.type === 'gm:disconnect') {
					gun.disconnect(relay);
					relayState = 'disconnected';
					sendToParent({ type: 'gm:status', frameId, status: 'disconnected' });
				} else if (msg.type === 'gm:reconnect') {
					gun.connect(relay);
					await waitForHandshake(gun, relay, 10_000);
					if (dirtyOffline) {
						gun.putText(soul, 'text', value);
						dirtyOffline = false;
					}
					gun.fetchSoul(soul);
					relayState = 'connected';
					sendToParent({ type: 'gm:status', frameId, status: 'connected' });
				}
			});

			status = 'ready';
			markReady();
		} catch (cause) {
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});

	function handleInput() {
		if (relayState === 'disconnected') dirtyOffline = true;
		gun?.putText(soul, 'text', value);
	}
</script>

<div
	class="flex min-h-screen flex-col items-center justify-center gap-3 bg-background p-6 text-foreground"
	data-relay-state={relayState}
>
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting gunmetal…</p>
	{:else if status === 'degraded'}
		<p class="text-sm text-destructive" data-testid="client-degraded">wasm failed to start.</p>
	{:else}
		<div class="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
			Session {frameId}
			<span
				class="rounded px-1.5 py-0.5 {relayState === 'connected'
					? 'bg-green-500/15 text-green-600 dark:text-green-400'
					: 'bg-amber-500/15 text-amber-600 dark:text-amber-400'}"
				data-testid="relay-badge"
			>
				{relayState === 'connected' ? 'online' : 'offline (local persistence active)'}
			</span>
		</div>
		<input
			data-testid="shared-input"
			class="w-full max-w-sm rounded-md border bg-background px-3 py-2 text-base shadow-sm focus:outline-2"
			bind:value
			oninput={handleInput}
			placeholder="Survives reloads…"
		/>
		{#if hint}
			<pre class="max-w-sm whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		{/if}
	{/if}
</div>

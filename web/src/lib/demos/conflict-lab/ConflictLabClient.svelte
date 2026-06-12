<script lang="ts">
	// Split-brain editor: the parent page disconnects both sessions from
	// the relay, each edits the same field independently, and on
	// reconnect HAM merges the divergent writes — both sides converge to
	// the same value with no referee.
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

	let { slug = 'conflict-lab' }: { slug?: string } = $props();

	let value = $state('');
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let relayState = $state<'connected' | 'disconnected'>('connected');
	let hint = $state('');
	let frameId = $state('');

	let gun: WasmGunInstance | undefined;
	let soul = '';
	let relay = '';
	// Only re-announce after reconnect if this side actually edited
	// while split — an idle side must not beat real edits with a
	// fresher-state replay of stale data.
	let dirtyOffline = false;

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		relay = params.relay;
		soul = `demo/${slug}/${params.room}`;

		try {
			gun = await bootGunmetal(params);
			gun.on(soul, 'text', (json: string) => {
				const incoming = JSON.parse(json);
				if (typeof incoming === 'string' && incoming !== value) {
					value = incoming;
				}
			});
			gun.fetchSoul(soul);

			onParentMessage(async (msg) => {
				if (!gun) return;
				if (msg.type === 'gm:disconnect') {
					gun.disconnect(relay);
					relayState = 'disconnected';
					sendToParent({ type: 'gm:status', frameId, status: 'disconnected' });
				} else if (msg.type === 'gm:reconnect') {
					gun.connect(relay);
					await waitForHandshake(gun, relay, 10_000);
					// Re-announce local edits: HAM decides the winner on
					// every peer identically.
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
			hint = relayRunHint(params.relay);
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
		<div class="max-w-sm rounded-lg border border-destructive/50 p-4 text-sm" data-testid="client-degraded">
			<p class="font-medium">Relay unreachable</p>
			<pre class="mt-2 whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		</div>
	{:else}
		<div class="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
			Session {frameId}
			<span
				class="rounded px-1.5 py-0.5 {relayState === 'connected'
					? 'bg-green-500/15 text-green-600 dark:text-green-400'
					: 'bg-destructive/15 text-destructive'}"
				data-testid="relay-badge"
			>
				{relayState}
			</span>
		</div>
		<input
			data-testid="shared-input"
			class="w-full max-w-sm rounded-md border bg-background px-3 py-2 text-base shadow-sm focus:outline-2"
			bind:value
			oninput={handleInput}
			placeholder="Edit while split…"
		/>
	{/if}
</div>

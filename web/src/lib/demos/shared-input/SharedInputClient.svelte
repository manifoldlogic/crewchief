<script lang="ts">
	import { onMount } from 'svelte';
	import {
		bootGunmetal,
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		relayRunHint,
		type WasmGunInstance
	} from '$lib/gun/client';
	import { sendToParent } from '$lib/gun/frame-protocol';

	let { slug = 'shared-input' }: { slug?: string } = $props();

	let value = $state('');
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');

	let gun: WasmGunInstance | undefined;
	let soul = '';

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		soul = `demo/${slug}/${params.room}`;

		try {
			gun = await bootGunmetal(params);

			// Subscribe BEFORE fetching so the answer always lands in the
			// callback; only then is this client "ready".
			gun.on(soul, 'text', (json: string) => {
				const incoming = JSON.parse(json);
				if (typeof incoming === 'string' && incoming !== value) {
					value = incoming;
				}
			});
			gun.fetchSoul(soul);

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
		gun?.putText(soul, 'text', value);
	}
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-3 bg-background p-6 text-foreground">
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting gunmetal…</p>
	{:else if status === 'degraded'}
		<div class="max-w-sm rounded-lg border border-destructive/50 p-4 text-sm" data-testid="client-degraded">
			<p class="font-medium">Relay unreachable</p>
			<pre class="mt-2 whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		</div>
	{:else}
		<label class="text-xs uppercase tracking-wide text-muted-foreground" for="shared">
			Session {frameId} — type here
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

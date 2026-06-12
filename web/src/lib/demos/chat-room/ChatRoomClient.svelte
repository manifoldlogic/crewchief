<script lang="ts">
	// Chat room: append-only collection under time-sortable UUID keys —
	// keys sort chronologically, so render order needs no clock sync.
	// History "pagination" filters client-side over the sorted keys
	// (wire-level LEX queries are future crate work; see the demo copy).
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

	let { slug = 'chat-room' }: { slug?: string } = $props();

	let draft = $state('');
	let raw = $state<Record<string, string>>({});
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');
	let limit = $state(50);

	let gun: WasmGunInstance | undefined;
	let soul = '';

	// Node subscriptions fire PER KEY with (value, key).
	const messages = $derived(
		Object.entries(raw)
			.sort(([a], [b]) => (a < b ? -1 : 1)) // uuid keys are time-sortable
			.slice(-limit)
			.map(([msgKey, v]) => {
				const [author, ...rest] = v.split('|');
				return { key: msgKey, author, text: rest.join('|') };
			})
	);

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		soul = `demo/${slug}/${params.room}`;

		try {
			gun = await bootGunmetal(params);
			gun.onNode(soul, (json: string, msgKey: string) => {
				const incoming = JSON.parse(json);
				if (typeof incoming === 'string') {
					raw = { ...raw, [msgKey]: incoming };
				}
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

	function send() {
		const text = draft.trim();
		if (!gun || !text) return;
		gun.setValue(soul, JSON.stringify(`${frameId}|${text}`));
		draft = '';
	}
</script>

<div class="flex min-h-screen flex-col bg-background p-4 text-foreground">
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting gunmetal…</p>
	{:else if status === 'degraded'}
		<div class="rounded-lg border border-destructive/50 p-4 text-sm" data-testid="client-degraded">
			<p class="font-medium">Relay unreachable</p>
			<pre class="mt-2 whitespace-pre-wrap text-xs text-muted-foreground">{hint}</pre>
		</div>
	{:else}
		<p class="text-xs uppercase tracking-wide text-muted-foreground">Session {frameId}</p>
		<ul class="my-2 flex-1 space-y-1 overflow-y-auto" data-testid="chat-messages">
			{#each messages as msg (msg.key)}
				<li class="text-sm">
					<span class="font-semibold {msg.author === frameId ? 'text-primary' : ''}">{msg.author}:</span>
					{msg.text}
				</li>
			{/each}
		</ul>
		<form
			class="flex gap-2"
			onsubmit={(event) => {
				event.preventDefault();
				send();
			}}
		>
			<input
				class="flex-1 rounded-md border bg-background px-3 py-2 text-sm"
				bind:value={draft}
				data-testid="chat-input"
				placeholder="Say something…"
			/>
			<button class="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground" type="submit" data-testid="chat-send">
				Send
			</button>
		</form>
	{/if}
</div>

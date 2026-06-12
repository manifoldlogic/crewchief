<script lang="ts">
	// ECDH direct messages: each session generates a keypair, publishes
	// only its PUBLIC encryption key, and derives the same shared secret
	// from (their epub, my epriv) — encrypt with it and only the two of
	// you can read what crosses the (public) graph.
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

	let { slug = 'secret-handshake' }: { slug?: string } = $props();

	let draft = $state('');
	let envelopes = $state<Record<string, { from: string; ct: string }>>({});
	let theirEpub = $state<string | null>(null);
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');

	let gun: WasmGunInstance | undefined;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let sea: any;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let myPair: any;
	let msgsSoul = '';

	const messages = $derived(
		Object.entries(envelopes)
			.sort(([a], [b]) => (a < b ? -1 : 1))
			.map(([msgKey, env]) => {
				if (!theirEpub || !myPair) return { key: msgKey, from: env.from, text: '…' };
				try {
					const shared = sea.secret(theirEpub, myPair.epriv) as string;
					const plain = sea.decrypt(env.ct, shared);
					return {
						key: msgKey,
						from: env.from,
						text: typeof plain === 'string' ? JSON.parse(plain) : '…'
					};
				} catch {
					return { key: msgKey, from: env.from, text: '🔒' };
				}
			})
	);

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		const keysSoul = `demo/${slug}/${params.room}/keys`;
		msgsSoul = `demo/${slug}/${params.room}/msgs`;

		try {
			gun = await bootGunmetal(params);
			sea = new (await wasmModule()).WasmSEA();
			myPair = JSON.parse(sea.pair());

			// Publish ONLY the public encryption key.
			gun.putText(keysSoul, frameId, myPair.epub);
			gun.onNode(keysSoul, (json: string, who: string) => {
				if (who === frameId) return;
				try {
					const epub = JSON.parse(json);
					if (typeof epub === 'string') theirEpub = epub;
				} catch {
					// malformed peer value — ignore
				}
			});
			gun.onNode(msgsSoul, (json: string, msgKey: string) => {
				try {
					const env = JSON.parse(json);
					if (typeof env === 'string') {
						const parsed = JSON.parse(env);
						if (parsed && typeof parsed.ct === 'string') {
							envelopes = { ...envelopes, [msgKey]: parsed };
						}
					}
				} catch {
					// malformed peer value — ignore
				}
			});
			gun.fetchSoul(keysSoul);
			gun.fetchSoul(msgsSoul);

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
		if (!gun || !theirEpub || !text) return;
		const shared = sea.secret(theirEpub, myPair.epriv) as string;
		const ct = sea.encrypt(JSON.stringify(text), shared) as string;
		gun.setValue(msgsSoul, JSON.stringify(JSON.stringify({ from: frameId, ct })));
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
		<p class="text-xs uppercase tracking-wide text-muted-foreground" data-testid="sh-status">
			Session {frameId} — {theirEpub ? 'secure channel ready' : 'waiting for the other session…'}
		</p>
		<ul class="my-2 flex-1 space-y-1 overflow-y-auto text-sm" data-testid="sh-messages">
			{#each messages as msg (msg.key)}
				<li>
					<span class="font-semibold {msg.from === frameId ? 'text-primary' : ''}">{msg.from}:</span>
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
				data-testid="sh-input"
				placeholder="Encrypted DM…"
				disabled={!theirEpub}
			/>
			<button
				class="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground"
				type="submit"
				data-testid="sh-send"
				disabled={!theirEpub}
			>
				Send
			</button>
		</form>
	{/if}
</div>

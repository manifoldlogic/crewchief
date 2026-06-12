<script lang="ts">
	// Privacy: the graph is public, the CONTENT isn't. Notes are
	// encrypted with a key derived from a passphrase (SEA work, fixed
	// salt = the room id so every session derives the same key) before
	// they ever touch the graph — the raw view shows what peers and
	// relays actually store.
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

	let { slug = 'private-notes' }: { slug?: string } = $props();

	let passphrase = $state('');
	let draft = $state('');
	let raw = $state<Record<string, string>>({});
	let key = $state<string | null>(null);
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');

	let gun: WasmGunInstance | undefined;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let sea: any;
	let soul = '';
	let salt = '';

	const ciphertexts = $derived(
		Object.entries(raw).sort(([a], [b]) => (a < b ? -1 : 1))
	);

	const decrypted = $derived(
		ciphertexts.map(([noteKey, ct]) => {
			if (!key) return { key: noteKey, text: null };
			try {
				const plain = sea?.decrypt(ct, key);
				return { key: noteKey, text: typeof plain === 'string' ? JSON.parse(plain) : null };
			} catch {
				return { key: noteKey, text: null };
			}
		})
	);

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		soul = `demo/${slug}/${params.room}/notes`;
		salt = params.room;

		try {
			gun = await bootGunmetal(params);
			sea = new (await wasmModule()).WasmSEA();

			gun.onNode(soul, (json: string, noteKey: string) => {
				const ct = JSON.parse(json);
				if (typeof ct === 'string') raw = { ...raw, [noteKey]: ct };
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

	function unlock() {
		// Same passphrase + same salt → same key, on every session.
		key = sea.work(passphrase, salt) as string;
	}

	function add() {
		const text = draft.trim();
		if (!gun || !key || !text) return;
		const ct = sea.encrypt(JSON.stringify(text), key) as string;
		gun.setValue(soul, JSON.stringify(ct));
		draft = '';
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
		{#if !key}
			<div class="flex gap-2">
				<input
					class="flex-1 rounded-md border bg-background px-3 py-2 text-sm"
					bind:value={passphrase}
					type="password"
					data-testid="pn-pass"
					placeholder="Passphrase…"
				/>
				<button class="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground" onclick={unlock} data-testid="pn-unlock">
					Unlock
				</button>
			</div>
		{:else}
			<form
				class="flex gap-2"
				onsubmit={(event) => {
					event.preventDefault();
					add();
				}}
			>
				<input
					class="flex-1 rounded-md border bg-background px-3 py-2 text-sm"
					bind:value={draft}
					data-testid="pn-note"
					placeholder="A private note…"
				/>
				<button class="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground" type="submit" data-testid="pn-add">
					Add
				</button>
			</form>
			<ul class="space-y-1 text-sm" data-testid="pn-notes">
				{#each decrypted as note (note.key)}
					<li>{note.text ?? '🔒 (different passphrase)'}</li>
				{/each}
			</ul>
		{/if}
		<div class="mt-auto">
			<h3 class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
				What the graph actually stores
			</h3>
			<ul class="mt-1 max-h-24 space-y-1 overflow-y-auto font-mono text-[10px] text-muted-foreground" data-testid="pn-raw">
				{#each ciphertexts as [ctKey, ct] (ctKey)}
					<li class="truncate">{ct}</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>

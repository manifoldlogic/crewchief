<script lang="ts">
	// Certificates: the owner signs a small grant ("this pub key may
	// write to this path") and publishes it. Writers attach the cert to
	// their signed entries; READERS verify — cert signature, scope, and
	// entry signature — before trusting anything. No server enforces
	// this; the math does. Frame a = owner, frame b = guest.
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

	let { slug = 'doc-permissions' }: { slug?: string } = $props();

	const DOC_PATH = 'shared-doc';

	let draft = $state('');
	let entries = $state<Record<string, { by: string; sig: string; cert: string | null }>>({});
	let ownerPub = $state<string | null>(null);
	let guestPub = $state<string | null>(null);
	let certJson = $state<string | null>(null);
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');

	let gun: WasmGunInstance | undefined;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let sea: any;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let cert: any;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let myPair: any;
	let idsSoul = '';
	let certSoul = '';
	let entriesSoul = '';

	const isOwner = $derived(frameId === 'a');

	const rendered = $derived(
		Object.entries(entries)
			.sort(([a], [b]) => (a < b ? -1 : 1))
			.map(([entryKey, env]) => {
				let text = '';
				let accepted = false;
				try {
					const verified = sea.verify(env.sig, env.by);
					text = JSON.parse(verified);
					accepted =
						env.cert !== null &&
						ownerPub !== null &&
						JSON.parse(env.cert).issuer === ownerPub &&
						(cert.grantsAccess(env.cert, env.by, DOC_PATH) as boolean);
				} catch {
					text = '(bad signature)';
				}
				return { key: entryKey, by: env.by, text, accepted };
			})
	);

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		idsSoul = `demo/${slug}/${params.room}/ids`;
		certSoul = `demo/${slug}/${params.room}/cert`;
		entriesSoul = `demo/${slug}/${params.room}/entries`;

		try {
			gun = await bootGunmetal(params);
			const wasm = await wasmModule();
			sea = new wasm.WasmSEA();
			cert = new wasm.WasmCert();
			myPair = JSON.parse(sea.pair());

			gun.putText(idsSoul, frameId, myPair.pub);
			gun.onNode(idsSoul, (json: string, who: string) => {
				try {
					const pub = JSON.parse(json);
					if (typeof pub !== 'string') return;
					if (who === 'a') ownerPub = pub;
					if (who === 'b') guestPub = pub;
				} catch {
					// malformed peer value — ignore
				}
			});
			gun.onNode(certSoul, (json: string, key: string) => {
				if (key !== 'guest') return;
				try {
					const value = JSON.parse(json);
					if (typeof value === 'string') certJson = value;
				} catch {
					// malformed peer value — ignore
				}
			});
			gun.onNode(entriesSoul, (json: string, entryKey: string) => {
				try {
					const value = JSON.parse(json);
					if (typeof value === 'string') {
						const env = JSON.parse(value);
						if (env && typeof env.sig === 'string') {
							entries = { ...entries, [entryKey]: env };
						}
					}
				} catch {
					// malformed peer value — ignore
				}
			});
			gun.fetchSoul(idsSoul);
			gun.fetchSoul(certSoul);
			gun.fetchSoul(entriesSoul);

			status = 'ready';
			markReady();
		} catch (cause) {
			hint = relayRunHint(params.relay);
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});

	function grant() {
		if (!gun || !guestPub) return;
		const created = cert.create(guestPub, DOC_PATH, undefined, myPair.pub, myPair.priv) as string;
		gun.putText(certSoul, 'guest', created);
	}

	function write() {
		const text = draft.trim();
		if (!gun || !text) return;
		const sig = sea.sign(JSON.stringify(text), myPair.priv, myPair.pub) as string;
		const envelope = JSON.stringify({ by: myPair.pub, sig, cert: certJson });
		gun.setValue(entriesSoul, JSON.stringify(envelope));
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
		<p class="text-xs uppercase tracking-wide text-muted-foreground">
			Session {frameId} — {isOwner ? 'document owner' : 'guest'}
		</p>

		{#if isOwner}
			<button
				class="mt-2 w-fit rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground disabled:opacity-50"
				onclick={grant}
				disabled={!guestPub || certJson !== null}
				data-testid="dp-grant"
			>
				{certJson ? 'Write access granted' : 'Grant guest write access'}
			</button>
		{:else}
			<p class="mt-1 text-xs" data-testid="dp-cert-status">
				{certJson ? 'You hold a write certificate' : 'No certificate yet — writes will be rejected by readers'}
			</p>
			<form
				class="mt-1 flex gap-2"
				onsubmit={(event) => {
					event.preventDefault();
					write();
				}}
			>
				<input
					class="flex-1 rounded-md border bg-background px-3 py-2 text-sm"
					bind:value={draft}
					data-testid="dp-input"
					placeholder="Write to the shared doc…"
				/>
				<button class="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground" type="submit" data-testid="dp-write">
					Write
				</button>
			</form>
		{/if}

		<h3 class="mt-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
			Shared document (as verified by THIS session)
		</h3>
		<ul class="mt-1 space-y-1 text-sm" data-testid="dp-entries">
			{#each rendered as entry (entry.key)}
				<li
					class="rounded border px-2 py-1 {entry.accepted ? '' : 'border-destructive/50 text-muted-foreground'}"
					data-status={entry.accepted ? 'accepted' : 'rejected'}
				>
					{entry.accepted ? '✓' : '✗ rejected (no valid certificate)'} — {entry.text}
				</li>
			{/each}
		</ul>
	{/if}
</div>

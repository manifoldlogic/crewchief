<script lang="ts">
	// Identity: SEA users. Signup derives a keypair and encrypts it with
	// a password-derived proof; login decrypts it; session restore feeds
	// the saved pair back via authPair — no password round-trip, no
	// server-side session.
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

	let { slug = 'login' }: { slug?: string } = $props();

	let alias = $state('');
	let password = $state('');
	let signedInAs = $state<string | null>(null);
	let restored = $state(false);
	let errorMsg = $state('');
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');
	let frameId = $state('');

	let gun: WasmGunInstance | undefined;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let user: any;
	let storageKey = '';

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		frameId = params.frameId;
		alias = `user-${params.frameId}-${params.room}`;
		storageKey = `gm-login-${params.room}-${params.frameId}`;

		try {
			gun = await bootGunmetal(params);
			const wasm = await wasmModule();
			user = new wasm.WasmUser(gun);

			// Session restore: a previously-saved pair re-authenticates
			// without the password.
			const savedPair = sessionStorage.getItem(storageKey);
			if (savedPair) {
				const result = JSON.parse(user.authPair(savedPair));
				if (!result.err) {
					signedInAs = alias;
					restored = true;
				}
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

	function signup() {
		errorMsg = '';
		const created = JSON.parse(user.create(alias, password));
		if (created.err) {
			errorMsg = created.err;
			return;
		}
		login();
	}

	function login() {
		errorMsg = '';
		const result = JSON.parse(user.auth(alias, password));
		if (result.err) {
			errorMsg = result.err;
			return;
		}
		signedInAs = alias;
		restored = false;
		const pair = user.pairJson();
		if (typeof pair === 'string') sessionStorage.setItem(storageKey, pair);
	}

	function logout() {
		user.leave();
		sessionStorage.removeItem(storageKey);
		signedInAs = null;
		restored = false;
		errorMsg = '';
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
	{:else if signedInAs}
		<p class="text-sm" data-testid="login-status">
			Signed in as <strong>{signedInAs}</strong>{#if restored}
				<span class="ml-1 rounded bg-green-500/15 px-1.5 py-0.5 text-xs text-green-600 dark:text-green-400" data-testid="login-restored">session restored</span>{/if}
		</p>
		<button class="rounded-md border px-3 py-1.5 text-sm" onclick={logout} data-testid="login-logout">
			Log out
		</button>
	{:else}
		<p class="text-xs uppercase tracking-wide text-muted-foreground">Session {frameId}</p>
		<p class="text-sm text-muted-foreground" data-testid="login-status">Signed out</p>
		<input class="w-full max-w-xs rounded-md border bg-background px-3 py-2 text-sm" bind:value={alias} data-testid="login-alias" placeholder="alias" />
		<input
			class="w-full max-w-xs rounded-md border bg-background px-3 py-2 text-sm"
			bind:value={password}
			data-testid="login-password"
			placeholder="password"
			type="password"
		/>
		<div class="flex gap-2">
			<button class="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground" onclick={signup} data-testid="login-signup">
				Sign up
			</button>
			<button class="rounded-md border px-3 py-1.5 text-sm" onclick={login} data-testid="login-login">
				Log in
			</button>
		</div>
		{#if errorMsg}
			<p class="max-w-xs text-sm text-destructive" data-testid="login-error">{errorMsg}</p>
		{/if}
	{/if}
</div>

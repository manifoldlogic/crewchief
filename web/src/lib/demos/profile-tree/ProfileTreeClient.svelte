<script lang="ts">
	// Nested profile editor: linked nodes assembled into one document via
	// load() (gun/lib/open with once), plus not() for honest empty states
	// (single-frame: the graph lives only in this session).
	import { onMount } from 'svelte';
	import {
		bootGunmetal,
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		type WasmGunInstance
	} from '$lib/gun/client';

	let { slug = 'profile-tree' }: { slug?: string } = $props();

	let name = $state('Ada');
	let city = $state('London');
	let employer = $state('Analytical Engines Ltd');
	let assembled = $state<string | null>(null);
	let lookup = $state('');
	let lookupResult = $state<'idle' | 'checking' | 'absent' | 'present'>('idle');
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');

	let gun: WasmGunInstance | undefined;
	let root = '';

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		root = `profile/${params.room}`;
		lookup = `profile/${params.room}-nobody`;

		try {
			gun = await bootGunmetal({ ...params, singleFrame: true });
			status = 'ready';
			markReady();
		} catch (cause) {
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});

	function save() {
		if (!gun) return;
		// Three nodes, two links — a document is a subtree of the graph.
		gun.putText(root, 'name', name);
		gun.putText(`${root}/address`, 'city', city);
		gun.putText(`${root}/employer`, 'name', employer);
		gun.putLink(root, 'address', `${root}/address`);
		gun.putLink(root, 'employer', `${root}/employer`);
		assembled = null;
	}

	function loadDocument() {
		// load() follows every link and hands back the full tree once.
		gun?.load(root, (json: string) => {
			assembled = JSON.stringify(JSON.parse(json), null, 2);
		});
	}

	async function checkLookup() {
		if (!gun) return;
		lookupResult = 'checking';
		const absent = await gun.notWithin(lookup, '', 400);
		lookupResult = absent ? 'absent' : 'present';
	}
</script>

<div class="min-h-screen bg-background p-4 text-foreground">
	{#if status === 'booting'}
		<p class="text-sm text-muted-foreground" data-testid="client-booting">Starting gunmetal…</p>
	{:else if status === 'degraded'}
		<p class="text-sm text-destructive" data-testid="client-degraded">wasm failed to start.</p>
	{:else}
		<div class="grid gap-4 md:grid-cols-2">
			<div class="space-y-2">
				<h2 class="text-sm font-semibold">Edit profile</h2>
				<label class="block text-xs text-muted-foreground" for="pt-name">Name</label>
				<input id="pt-name" class="w-full rounded border bg-background px-2 py-1 text-sm" bind:value={name} data-testid="pt-name" />
				<label class="block text-xs text-muted-foreground" for="pt-city">Address → city (linked node)</label>
				<input id="pt-city" class="w-full rounded border bg-background px-2 py-1 text-sm" bind:value={city} data-testid="pt-city" />
				<label class="block text-xs text-muted-foreground" for="pt-employer">Employer → name (linked node)</label>
				<input id="pt-employer" class="w-full rounded border bg-background px-2 py-1 text-sm" bind:value={employer} data-testid="pt-employer" />
				<div class="flex gap-2 pt-1">
					<button class="rounded bg-primary px-3 py-1 text-sm text-primary-foreground" onclick={save} data-testid="pt-save">Save</button>
					<button class="rounded border px-3 py-1 text-sm" onclick={loadDocument} data-testid="pt-load">Load full document</button>
				</div>
				<h2 class="pt-3 text-sm font-semibold">Look up a profile</h2>
				<div class="flex gap-2">
					<input class="flex-1 rounded border bg-background px-2 py-1 text-sm" bind:value={lookup} data-testid="pt-lookup" />
					<button class="rounded border px-3 py-1 text-sm" onclick={checkLookup} data-testid="pt-check">Check</button>
				</div>
				{#if lookupResult === 'absent'}
					<p class="rounded border border-dashed p-2 text-sm text-muted-foreground" data-testid="pt-empty-state">
						No such profile — <code>not()</code> fired. (Absence can't be guaranteed in a
						distributed system; this means "nothing found here, yet".)
					</p>
				{:else if lookupResult === 'present'}
					<p class="text-sm" data-testid="pt-found">Profile exists.</p>
				{/if}
			</div>
			<div>
				<h2 class="text-sm font-semibold">Assembled document</h2>
				<pre class="mt-2 overflow-x-auto rounded border bg-muted/30 p-2 text-xs" data-testid="pt-document">{assembled ?? '(save, then load)'}</pre>
			</div>
		</div>
	{/if}
</div>

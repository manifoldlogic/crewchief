<script lang="ts">
	// Single-session graph explorer: put values, follow links, watch
	// subscriptions fire — the graph data model with no network at all
	// (?frames=1: no relay, no readiness dependency on a handshake).
	import { onMount } from 'svelte';
	import {
		bootGunmetal,
		installReadyPromise,
		markDegraded,
		markReady,
		parseClientParams,
		type WasmGunInstance
	} from '$lib/gun/client';

	let { slug = 'graph-explorer' }: { slug?: string } = $props();

	let soul = $state('people/mark');
	let key = $state('name');
	let value = $state('Mark');
	let nodeJson = $state<string | null>(null);
	let log = $state<string[]>([]);
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');

	let gun: WasmGunInstance | undefined;
	let nodeListener: number | undefined;
	let watchedSoul = $state('');

	function watch(target: string) {
		if (!gun) return;
		if (nodeListener !== undefined && watchedSoul) {
			// onNode listeners are keyed per soul; re-subscribe on change.
			gun.off(watchedSoul, '', nodeListener);
		}
		watchedSoul = target;
		nodeJson = gun.getNode(target) as string | null;
		// Per-key fires; refresh the full node view from the graph.
		nodeListener = gun.onNode(target, (json: string, changedKey: string) => {
			nodeJson = gun?.getNode(target) as string | null;
			log = [...log, `update ${target}.${changedKey} = ${json}`].slice(-20);
		});
	}

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		void slug;
		try {
			gun = await bootGunmetal({ ...params, singleFrame: true });
			watch(soul);
			status = 'ready';
			markReady();
		} catch (cause) {
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});

	function doPut() {
		if (!gun) return;
		gun.putText(soul, key, value);
		log = [...log, `put ${soul}.${key} = ${JSON.stringify(value)}`].slice(-20);
		if (soul !== watchedSoul) watch(soul);
	}

	function doLink() {
		if (!gun) return;
		// Link the watched node from an index node — graphs are made of
		// souls pointing at souls.
		gun.putLink('index', soul.replaceAll('/', ':'), soul);
		log = [...log, `link index.${soul.replaceAll('/', ':')} → ${soul}`].slice(-20);
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
				<h2 class="text-sm font-semibold">Write</h2>
				<input class="w-full rounded border bg-background px-2 py-1 text-sm" bind:value={soul} data-testid="ge-soul" placeholder="soul" />
				<div class="flex gap-2">
					<input class="w-1/2 rounded border bg-background px-2 py-1 text-sm" bind:value={key} data-testid="ge-key" placeholder="key" />
					<input class="w-1/2 rounded border bg-background px-2 py-1 text-sm" bind:value data-testid="ge-value" placeholder="value" />
				</div>
				<div class="flex gap-2">
					<button class="rounded bg-primary px-3 py-1 text-sm text-primary-foreground" onclick={doPut} data-testid="ge-put">Put</button>
					<button class="rounded border px-3 py-1 text-sm" onclick={doLink} data-testid="ge-link">Link from index</button>
				</div>
				<h2 class="pt-2 text-sm font-semibold">Subscription log</h2>
				<ul class="max-h-40 space-y-1 overflow-y-auto text-xs text-muted-foreground" data-testid="ge-log">
					{#each log as line, i (i)}
						<li>{line}</li>
					{/each}
				</ul>
			</div>
			<div>
				<h2 class="text-sm font-semibold">Node <code class="text-xs">{watchedSoul}</code></h2>
				<pre class="mt-2 overflow-x-auto rounded border bg-muted/30 p-2 text-xs" data-testid="ge-inspector">{nodeJson ?? '(empty — put something)'}</pre>
			</div>
		</div>
	{/if}
</div>

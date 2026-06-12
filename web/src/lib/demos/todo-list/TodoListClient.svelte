<script lang="ts">
	// Collaborative todo list: items are linked nodes in a set; toggling
	// writes the item node; removal is unset() — the link is nulled, the
	// item node itself survives (a documented GUN semantic).
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

	let { slug = 'todo-list' }: { slug?: string } = $props();

	let draft = $state('');
	let items = $state<Record<string, { text: string; done: boolean }>>({});
	let status = $state<'booting' | 'ready' | 'degraded'>('booting');
	let hint = $state('');

	let gun: WasmGunInstance | undefined;
	let setSoul = '';
	const watched = new Set<string>();

	const sorted = $derived(
		Object.entries(items).sort(([a], [b]) => (a < b ? -1 : 1))
	);

	function mergeItemField(itemSoul: string, field: string, json: string) {
		const incoming = JSON.parse(json);
		const current = items[itemSoul] ?? { text: '', done: false };
		if (field === 'text' && typeof incoming === 'string') {
			items = { ...items, [itemSoul]: { ...current, text: incoming } };
		} else if (field === 'done' && typeof incoming === 'boolean') {
			items = { ...items, [itemSoul]: { ...current, done: incoming } };
		}
	}

	function watchItem(itemSoul: string) {
		if (!gun || watched.has(itemSoul)) return;
		watched.add(itemSoul);
		// Per-key fires: (value, key) for each field of the item node.
		gun.onNode(itemSoul, (json: string, field: string) => {
			mergeItemField(itemSoul, field, json);
		});
		// The item's data may have arrived BEFORE this subscription (the
		// set-link event races the item put, and a GET answer that merges
		// as a no-op fires no events) — read current state explicitly.
		const existing = gun.getNode(itemSoul) as string | null;
		if (existing) {
			for (const [field, v] of Object.entries(JSON.parse(existing) as Record<string, unknown>)) {
				mergeItemField(itemSoul, field, JSON.stringify(v));
			}
		}
		gun.fetchSoul(itemSoul);
	}

	// The set node fires per key: key = item soul, value = link (or null
	// after unset()).
	function onSetUpdate(json: string, itemKey: string) {
		const link = JSON.parse(json);
		if (link === null) {
			if (itemKey in items) {
				const next = { ...items };
				delete next[itemKey];
				items = next;
			}
		} else {
			watchItem(itemKey);
		}
	}

	onMount(async () => {
		installReadyPromise();
		const params = parseClientParams(window.location);
		setSoul = `demo/${slug}/${params.room}`;

		try {
			gun = await bootGunmetal(params);
			gun.onNode(setSoul, onSetUpdate);
			gun.fetchSoul(setSoul);
			status = 'ready';
			markReady();
		} catch (cause) {
			hint = relayRunHint(params.relay);
			status = 'degraded';
			markDegraded();
			console.error(cause);
		}
	});

	function add() {
		const text = draft.trim();
		if (!gun || !text) return;
		gun.setObject(setSoul, JSON.stringify({ text, done: false }));
		draft = '';
	}

	function toggle(itemSoul: string) {
		if (!gun) return;
		gun.putBool(itemSoul, 'done', !items[itemSoul]?.done);
	}

	function remove(itemSoul: string) {
		if (!gun) return;
		gun.unset(setSoul, itemSoul);
		const next = { ...items };
		delete next[itemSoul];
		items = next;
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
				data-testid="todo-input"
				placeholder="Add a todo…"
			/>
			<button class="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground" type="submit" data-testid="todo-add">
				Add
			</button>
		</form>
		<ul class="mt-3 space-y-1" data-testid="todo-items">
			{#each sorted as [itemSoul, item] (itemSoul)}
				<li class="flex items-center gap-2 rounded border px-2 py-1 text-sm" data-done={item.done}>
					<input
						type="checkbox"
						checked={item.done}
						onchange={() => toggle(itemSoul)}
						data-testid="todo-toggle"
					/>
					<span class={item.done ? 'text-muted-foreground line-through' : ''}>{item.text}</span>
					<button
						class="ml-auto text-xs text-muted-foreground hover:text-destructive"
						onclick={() => remove(itemSoul)}
						data-testid="todo-remove"
					>
						remove
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

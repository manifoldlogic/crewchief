<script lang="ts">
	import { onMount } from 'svelte';
	import { sendToFrame } from '$lib/gun/frame-protocol';

	let {
		slug,
		frames = 2,
		engines = {},
		connectivityControls = false
	}: {
		slug: string;
		frames?: number;
		engines?: Record<string, string>;
		connectivityControls?: boolean;
	} = $props();

	let frameEls = $state<Record<string, HTMLIFrameElement | undefined>>({});

	function broadcast(type: 'gm:disconnect' | 'gm:reconnect') {
		for (const el of Object.values(frameEls)) {
			if (el) sendToFrame(el, { type });
		}
	}

	// Room + relay resolve at runtime (static-safe): ?room= and ?relay=
	// on the demo page override; otherwise a fresh room id per visit and
	// the default relay on the page's host (spec §5.4).
	let room = $state('');
	let relay = $state('');

	onMount(() => {
		const q = new URLSearchParams(window.location.search);
		room = q.get('room') ?? Math.random().toString(36).slice(2, 10);
		relay = q.get('relay') ?? `ws://${window.location.hostname}:8765/gun`;
	});

	const frameIds = ['a', 'b', 'c', 'd'];

	function clientSrc(frameId: string): string {
		const engine = engines[frameId] ? `&engine=${engines[frameId]}` : '';
		// Single-frame stages run relay-less (learn-chapter variants).
		const single = frames === 1 ? '&frames=1' : '';
		return `/gunmetal/demos/${slug}/client?room=${room}&frameId=${frameId}&relay=${encodeURIComponent(relay)}${engine}${single}`;
	}
</script>

{#if room}
	{#if connectivityControls}
		<div class="mb-3 flex gap-2">
			<button
				class="rounded-md border px-3 py-1.5 text-sm"
				onclick={() => broadcast('gm:disconnect')}
				data-testid="stage-disconnect"
			>
				Disconnect both from relay
			</button>
			<button
				class="rounded-md border px-3 py-1.5 text-sm"
				onclick={() => broadcast('gm:reconnect')}
				data-testid="stage-reconnect"
			>
				Reconnect
			</button>
		</div>
	{/if}
	<div class="grid gap-4 {frames > 1 ? 'md:grid-cols-2' : ''}" data-testid="demo-frames">
		{#each frameIds.slice(0, frames) as frameId (frameId)}
			<iframe
				bind:this={frameEls[frameId]}
				title="session {frameId}"
				data-testid="frame-{frameId}"
				src={clientSrc(frameId)}
				class="h-72 w-full rounded-lg border bg-background"
			></iframe>
		{/each}
	</div>
	<p class="mt-2 text-xs text-muted-foreground">
		Each pane is an isolated session syncing through the relay.
		<a class="underline" href={clientSrc('tab')} target="_blank" data-testid="open-client-tab">
			Open this client in a new tab
		</a>
		to prove it across windows or devices.
	</p>
{/if}

<script lang="ts">
	import { onMount } from 'svelte';

	let {
		slug,
		frames = 2
	}: {
		slug: string;
		frames?: number;
	} = $props();

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
		return `/gunmetal/demos/${slug}/client?room=${room}&frameId=${frameId}&relay=${encodeURIComponent(relay)}`;
	}
</script>

{#if room}
	<div class="grid gap-4 {frames > 1 ? 'md:grid-cols-2' : ''}" data-testid="demo-frames">
		{#each frameIds.slice(0, frames) as frameId (frameId)}
			<iframe
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

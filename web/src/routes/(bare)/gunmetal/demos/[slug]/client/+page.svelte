<script lang="ts">
	// Bare demo client page — no shell chrome. Booted inside demo-page
	// iframes (and openable directly). Runtime query params (static-safe):
	//   ?room=    shared room id        ?frameId=  storage namespace
	//   ?relay=   relay ws url          ?engine=   gunmetal | gun
	//   ?frames=1 single-frame variant (no relay)
	// Client components own the §5.5 readiness contract (data-ready).
	import { clientRegistry } from '$lib/demos/registry';

	let { data } = $props();

	const Client = $derived(clientRegistry[data.demo.slug]);
</script>

<svelte:head><title>{data.demo.title} — client</title></svelte:head>

{#if Client}
	<Client slug={data.demo.slug} />
{:else}
	<div class="flex min-h-screen items-center justify-center bg-background p-4 text-foreground">
		<p class="text-sm text-muted-foreground" data-testid="client-placeholder">
			{data.demo.title} client — coming in a later phase.
		</p>
	</div>
{/if}

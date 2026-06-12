<script lang="ts">
	// Bare demo client page — no shell chrome. Booted inside demo-page
	// iframes (and openable directly). Runtime query params (static-safe):
	//   ?room=    shared room id        ?frameId=  storage namespace
	//   ?relay=   relay ws url          ?engine=   gunmetal | gun
	//   ?frames=1 single-frame variant (no relay)
	// The wasm boot + readiness contract (data-ready) land in phase 2.
	let { data } = $props();
</script>

<svelte:head><title>{data.demo.title} — client</title></svelte:head>

<div class="flex min-h-screen items-center justify-center bg-background p-4 text-foreground">
	<p class="text-sm text-muted-foreground" data-testid="client-placeholder">
		{data.demo.title} client — wasm boot coming in the next phase.
	</p>
</div>

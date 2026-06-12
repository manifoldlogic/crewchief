<script lang="ts">
	import { capabilityById, demosForGrid } from '$lib/catalog';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
</script>

<svelte:head><title>Demos — Gunmetal</title></svelte:head>

<h1 class="text-3xl font-bold tracking-tight">Demo catalog</h1>
<p class="mt-3 max-w-2xl text-muted-foreground">
	Every gunmetal capability, shown as the web pattern it exists to enable. Each demo runs real
	wasm clients against a real relay — they double as the project's end-to-end tests.
</p>

<div class="mt-8 grid gap-4 sm:grid-cols-2" data-testid="demo-grid">
	{#each demosForGrid() as demo (demo.slug)}
		<a href={'/gunmetal/demos/' + demo.slug} class="group" data-demo={demo.slug}>
			<Card.Root class="h-full transition-colors group-hover:border-foreground/30">
				<Card.Header>
					<Card.Title class="flex items-center gap-2 text-base">
						{demo.title}
						{#if demo.flagship}<Badge data-testid="flagship-badge">flagship</Badge>{/if}
					</Card.Title>
					<Card.Description>{demo.pattern}</Card.Description>
				</Card.Header>
				<Card.Content>
					<span class="text-xs text-muted-foreground">
						{capabilityById.get(demo.capability)?.title}
					</span>
				</Card.Content>
			</Card.Root>
		</a>
	{/each}
</div>

<script lang="ts">
	import * as Card from '$lib/components/ui/card';

	const products = [
		{
			href: '/gunmetal',
			title: 'Gunmetal',
			blurb:
				'A Rust + WASM decentralized graph database that speaks the GUN.js wire protocol. Interactive demos, a guided course, and full API reference.',
			ready: true
		},
		{
			href: '/maproom',
			title: 'Maproom',
			blurb: 'Code indexing, hybrid search, and the daemon that powers CrewChief tooling.',
			ready: false
		},
		{
			href: '/crewchief',
			title: 'CrewChief CLI',
			blurb: 'Worktree and agent management for multi-branch development.',
			ready: false
		}
	];
</script>

<svelte:head><title>crewchief docs</title></svelte:head>

<div class="mx-auto max-w-5xl px-4 py-16">
	<h1 class="text-4xl font-bold tracking-tight">Documentation</h1>
	<p class="mt-3 max-w-2xl text-lg text-muted-foreground">
		Interactive, demo-driven documentation for the tools in this repository.
	</p>

	<div class="mt-10 grid gap-6 sm:grid-cols-2 lg:grid-cols-3" data-testid="product-cards">
		{#each products as product (product.href)}
			<a href={product.href} class="group">
				<Card.Root class="h-full transition-colors group-hover:border-foreground/30">
					<Card.Header>
						<Card.Title class="flex items-center gap-2">
							{product.title}
							{#if !product.ready}
								<span class="rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">soon</span>
							{/if}
						</Card.Title>
						<Card.Description>{product.blurb}</Card.Description>
					</Card.Header>
				</Card.Root>
			</a>
		{/each}
	</div>
</div>

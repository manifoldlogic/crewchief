<script lang="ts">
	import { orderedChapters } from '$lib/catalog';
</script>

<svelte:head><title>Learn — Gunmetal</title></svelte:head>

<h1 class="text-3xl font-bold tracking-tight">Learn gunmetal</h1>
<p class="mt-3 max-w-2xl text-muted-foreground">
	A guided path from "what even is a graph database" to running on the same mesh as GUN.js.
	By the end you'll understand the graph data model, realtime sync, CRDT conflict resolution,
	identity and encryption, permissions, and persistence — each taught through a working demo.
</p>
<p class="mt-2 text-sm text-muted-foreground">
	<strong>Prerequisites:</strong> comfortable reading JavaScript. No CRDT or distributed-systems
	knowledge assumed.
</p>

<ol class="mt-8 space-y-3" data-testid="chapter-list">
	{#each orderedChapters as chapter (chapter.slug)}
		<li>
			<a
				href={'/gunmetal/learn/' + chapter.slug}
				class="block rounded-lg border p-4 transition-colors hover:border-foreground/30"
			>
				<span class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
					Chapter {chapter.num}
				</span>
				<span class="block font-medium">{chapter.title}</span>
			</a>
		</li>
	{/each}
</ol>

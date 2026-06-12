<script lang="ts">
	import { modules } from '$lib/catalog';
</script>

<svelte:head><title>Reference — Gunmetal</title></svelte:head>

<h1 class="text-3xl font-bold tracking-tight">API reference</h1>
<p class="mt-3 max-w-2xl text-muted-foreground">
	Module-by-module reference for every public module of <code>crates/gunmetal</code>.
</p>

<ul class="mt-8 grid gap-2 sm:grid-cols-2" data-testid="module-list">
	{#each modules as moduleRef (moduleRef.name)}
		<li>
			<a
				href={'/gunmetal/reference/' + moduleRef.name}
				class="block rounded-lg border p-3 transition-colors hover:border-foreground/30"
			>
				<span class="font-mono text-sm font-medium">{moduleRef.name}</span>
				<span class="block text-sm text-muted-foreground">{moduleRef.purpose}</span>
			</a>
		</li>
	{/each}
</ul>

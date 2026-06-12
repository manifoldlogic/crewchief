<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';

	let { data } = $props();

	const surfaceLabel = {
		both: 'Rust + JS (wasm)',
		'native-only': 'native only',
		'wasm-only': 'wasm only'
	} as const;
</script>

<svelte:head><title>{data.moduleRef.name} — Gunmetal reference</title></svelte:head>

<div class="flex items-center gap-3">
	<h1 class="font-mono text-3xl font-bold tracking-tight">{data.moduleRef.name}</h1>
	<Badge variant="outline" data-testid="surface-badge">{surfaceLabel[data.moduleRef.surface]}</Badge>
</div>
<p class="mt-2 text-muted-foreground">{data.moduleRef.purpose}</p>

<div class="prose prose-neutral mt-6 dark:prose-invert">
	<p>
		<em>Full per-item reference (signatures, defaults, errors, wasm-bound names, examples,
		caveats) lands in the reference phase. Until then, see the rustdoc in
		<code>crates/gunmetal/src/{data.moduleRef.name}</code>.</em>
	</p>
</div>

{#if data.demos.length > 0}
	<section class="mt-8" data-testid="module-demos">
		<h2 class="text-lg font-semibold">Demos using this module</h2>
		<ul class="mt-2 list-disc pl-6 text-sm">
			{#each data.demos as demo (demo.slug)}
				<li><a class="underline" href={'/gunmetal/demos/' + demo.slug}>{demo.title}</a></li>
			{/each}
		</ul>
	</section>
{/if}

<script lang="ts">
	import { capabilityById } from '$lib/catalog';
	import * as Alert from '$lib/components/ui/alert';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';

	let { data } = $props();
</script>

<svelte:head><title>{data.demo.title} — Gunmetal demos</title></svelte:head>

<div class="flex items-center gap-2">
	<h1 class="text-3xl font-bold tracking-tight">{data.demo.title}</h1>
	{#if data.demo.flagship}<Badge>flagship</Badge>{/if}
</div>
<p class="mt-2 text-muted-foreground">{data.demo.pattern}</p>

<!-- (a) live demo — implemented per phase; placeholder until then -->
<section class="mt-8" data-testid="demo-stage">
	<Alert.Root>
		<Alert.Title>Demo coming online</Alert.Title>
		<Alert.Description>
			This demo is being built phase by phase — the live clients will appear here, backed by
			the real <code>gunmetal-relay</code>.
		</Alert.Description>
	</Alert.Root>
</section>

<Separator class="my-8" />

<!-- (f) manifest-generated triangle links -->
<section data-testid="demo-related">
	<h2 class="text-lg font-semibold">Related</h2>
	<ul class="mt-2 space-y-1 text-sm">
		<li>
			Capability:
			<span class="text-muted-foreground">
				{capabilityById.get(data.demo.capability)?.title}
			</span>
		</li>
		{#if data.chapters.length > 0}
			<li>
				Learn the concept:
				{#each data.chapters as chapter, i (chapter.slug)}{#if i > 0},
					{/if}<a class="underline" href={'/gunmetal/learn/' + chapter.slug}>{chapter.num}. {chapter.label}</a>{/each}
			</li>
		{/if}
		<li>
			Reference:
			{#each data.demo.modules as moduleName, i (moduleName)}{#if i > 0},
				{/if}<a class="underline" href={'/gunmetal/reference/' + moduleName}>{moduleName}</a>{/each}
		</li>
		<li>
			<a class="underline" href={'/gunmetal/demos/' + data.demo.slug + '/client'} data-testid="open-client">
				Open a bare client in a new tab
			</a>
			<span class="text-muted-foreground">(prove the sync is real across tabs or devices)</span>
		</li>
	</ul>
</section>

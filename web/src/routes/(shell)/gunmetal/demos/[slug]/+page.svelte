<script lang="ts">
	import { capabilityById } from '$lib/catalog';
	import DemoStage from '$lib/demos/DemoStage.svelte';
	import { demoContent } from '$lib/demos/content';
	import { implementedDemos } from '$lib/demos/implemented';
	import * as Alert from '$lib/components/ui/alert';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';

	let { data } = $props();

	const isLive = $derived(implementedDemos.includes(data.demo.slug));
	const content = $derived(demoContent[data.demo.slug]);
</script>

<svelte:head><title>{data.demo.title} — Gunmetal demos</title></svelte:head>

<div class="flex items-center gap-2">
	<h1 class="text-3xl font-bold tracking-tight">{data.demo.title}</h1>
	{#if data.demo.flagship}<Badge>flagship</Badge>{/if}
</div>
<p class="mt-2 text-muted-foreground">{data.demo.pattern}</p>

<!-- (a) live demo — implemented per phase; placeholder until then -->
<section class="mt-8" data-testid="demo-stage">
	{#if isLive}
		<DemoStage
			slug={data.demo.slug}
			frames={data.demo.singleFrame ? 1 : 2}
			engines={data.demo.engines ?? {}}
			connectivityControls={data.demo.connectivityControls ?? false}
		/>
	{:else}
		<Alert.Root>
			<Alert.Title>Demo coming online</Alert.Title>
			<Alert.Description>
				This demo is being built phase by phase — the live clients will appear here, backed by
				the real <code>gunmetal-relay</code>.
			</Alert.Description>
		</Alert.Root>
	{/if}
</section>

{#if content}
	<!-- (b) why this pattern exists -->
	<section class="mt-8" data-testid="demo-why">
		<h2 class="text-lg font-semibold">Why this pattern</h2>
		<p class="mt-2 max-w-3xl text-sm text-muted-foreground">{content.why}</p>
	</section>

	<!-- (c) minimal copy-paste snippets (no site glue) -->
	<section class="mt-8" data-testid="demo-snippets">
		<h2 class="text-lg font-semibold">Use it yourself</h2>
		<p class="mt-1 text-xs text-muted-foreground">
			Runnable against a local relay — no site harness required. The full client source
			(including the demo's iframe/room glue, which yours won't need) lives in
			<code>web/src/lib/demos/{data.demo.slug}/</code>.
		</p>
		<div class="mt-3 space-y-4">
			{#each content.snippets as snippet (snippet.label)}
				<div>
					<h3 class="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
						{snippet.label}
					</h3>
					<pre class="overflow-x-auto rounded-lg border bg-muted/40 p-3 text-xs"><code>{snippet.code}</code></pre>
				</div>
			{/each}
		</div>
	</section>

	<!-- (e) gotchas & limits -->
	<section class="mt-8" data-testid="demo-gotchas">
		<h2 class="text-lg font-semibold">Gotchas &amp; limits</h2>
		<ul class="mt-2 list-disc space-y-1 pl-6 text-sm text-muted-foreground">
			{#each content.gotchas as gotcha, i (i)}
				<li>{gotcha}</li>
			{/each}
		</ul>
	</section>
{/if}

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

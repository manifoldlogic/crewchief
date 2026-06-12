<script lang="ts">
	import { demoBySlug } from '$lib/catalog';
	import DemoStage from '$lib/demos/DemoStage.svelte';
	import { implementedDemos } from '$lib/demos/implemented';
	import { chapterContent } from '$lib/learn/content';
	import { Separator } from '$lib/components/ui/separator';

	let { data } = $props();

	const Content = $derived(chapterContent[data.chapter.slug]);

	// Single-frame variants before the sync chapter (num 5): nothing
	// networked appears before peers are taught (spec §3.3).
	const frameCount = (slug: string) =>
		data.chapter.num < 5 || demoBySlug.get(slug)?.singleFrame ? 1 : 2;
</script>

<svelte:head><title>{data.chapter.title} — Learn gunmetal</title></svelte:head>

<p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground" data-testid="chapter-progress">
	Chapter {data.chapter.num} of {data.total - 1}
</p>
<h1 class="mt-1 text-3xl font-bold tracking-tight">{data.chapter.title}</h1>

<div class="mt-6">
	{#if Content}
		<Content />
	{:else}
		<div class="prose prose-neutral dark:prose-invert">
			<p>
				<em>This chapter's full content lands with its demo phase. The structure below is
				generated from the catalog manifest.</em>
			</p>
		</div>
	{/if}
</div>

{#if data.chapter.embeds.length > 0}
	<section class="mt-8 space-y-6" data-testid="chapter-embeds">
		{#each data.chapter.embeds as slug (slug)}
			{@const demo = demoBySlug.get(slug)}
			<div>
				<h3 class="mb-2 text-sm font-semibold">
					{demo?.title ?? slug}
					<a class="ml-2 text-xs font-normal text-muted-foreground underline" href={'/gunmetal/demos/' + slug}>
						full demo page →
					</a>
				</h3>
				{#if implementedDemos.includes(slug)}
					<DemoStage {slug} frames={frameCount(slug)} engines={demo?.engines ?? {}} />
				{:else}
					<p class="rounded border border-dashed p-3 text-sm text-muted-foreground">
						This demo comes online in a later phase.
					</p>
				{/if}
			</div>
		{/each}
	</section>
{/if}

<Separator class="my-8" />

<!-- Required per-chapter footer (spec §3.3): prev/next + triangle links -->
<footer data-testid="chapter-footer">
	<p class="text-sm" data-testid="chapter-recap">
		<span class="font-semibold">What you can now do:</span>
		<span class="text-muted-foreground">{data.chapter.recap}</span>
	</p>
	{#if data.chapter.refs.length > 0}
		<p class="text-sm text-muted-foreground">
			Reference for this chapter:
			{#each data.chapter.refs as ref, i (ref)}{#if i > 0},
				{/if}<a class="underline" href={'/gunmetal/reference/' + ref}>{ref}</a>{/each}
		</p>
	{/if}
	<div class="mt-4 flex justify-between text-sm">
		{#if data.prev}
			<a class="underline" href={'/gunmetal/learn/' + data.prev.slug} data-testid="chapter-prev">
				← {data.prev.num}. {data.prev.label}
			</a>
		{:else}
			<a class="underline" href="/gunmetal/learn">← Course overview</a>
		{/if}
		{#if data.next}
			<a class="underline" href={'/gunmetal/learn/' + data.next.slug} data-testid="chapter-next">
				{data.next.num}. {data.next.label} →
			</a>
		{:else}
			<a class="underline" href="/gunmetal/demos" data-testid="chapter-next">
				You made it — explore all the demos →
			</a>
		{/if}
	</div>
</footer>

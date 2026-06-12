<script lang="ts">
	import { demoBySlug } from '$lib/catalog';
	import { Separator } from '$lib/components/ui/separator';

	let { data } = $props();
</script>

<svelte:head><title>{data.chapter.title} — Learn gunmetal</title></svelte:head>

<p class="text-xs font-semibold uppercase tracking-wide text-muted-foreground" data-testid="chapter-progress">
	Chapter {data.chapter.num} of {data.total - 1}
</p>
<h1 class="mt-1 text-3xl font-bold tracking-tight">{data.chapter.title}</h1>

<div class="prose prose-neutral mt-6 dark:prose-invert">
	<p>
		<em>This chapter's full content lands with its demo phase. The structure below is generated
		from the catalog manifest.</em>
	</p>
</div>

{#if data.chapter.embeds.length > 0}
	<section class="mt-8" data-testid="chapter-embeds">
		<h2 class="text-lg font-semibold">Try it</h2>
		<ul class="mt-2 list-disc pl-6 text-sm">
			{#each data.chapter.embeds as slug (slug)}
				<li>
					<a class="underline" href={'/gunmetal/demos/' + slug}>
						{demoBySlug.get(slug)?.title ?? slug}
					</a>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<Separator class="my-8" />

<!-- Required per-chapter footer (spec §3.3): prev/next + triangle links -->
<footer data-testid="chapter-footer">
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

<script lang="ts">
	import { referenceItems } from '$lib/reference';
	import { Badge } from '$lib/components/ui/badge';
	import * as Tabs from '$lib/components/ui/tabs';

	let { data } = $props();

	const surfaceLabel = {
		both: 'Rust + JS (wasm)',
		'native-only': 'native only',
		'wasm-only': 'wasm only'
	} as const;

	const items = $derived(referenceItems[data.moduleRef.name] ?? []);
</script>

<svelte:head><title>{data.moduleRef.name} — Gunmetal reference</title></svelte:head>

<div class="flex items-center gap-3">
	<h1 class="font-mono text-3xl font-bold tracking-tight">{data.moduleRef.name}</h1>
	<Badge variant="outline" data-testid="surface-badge">{surfaceLabel[data.moduleRef.surface]}</Badge>
</div>
<p class="mt-2 text-muted-foreground">{data.moduleRef.purpose}</p>

{#if items.length > 0}
	<div class="mt-8 space-y-8" data-testid="ref-items">
		{#each items as item (item.name)}
			<section class="rounded-lg border p-4">
				<h2 class="font-mono text-base font-semibold">{item.name}</h2>
				<pre class="mt-2 overflow-x-auto rounded bg-muted/40 p-2 text-xs"><code>{item.signature}</code></pre>
				<dl class="mt-2 space-y-1 text-sm">
					{#if item.wasmName}
						<div><dt class="inline font-medium">JS (wasm):</dt> <dd class="inline font-mono text-xs">{item.wasmName}</dd></div>
					{/if}
					{#if item.params}
						<div><dt class="inline font-medium">Parameters:</dt> <dd class="inline text-muted-foreground">{item.params}</dd></div>
					{/if}
					{#if item.returns}
						<div><dt class="inline font-medium">Behavior:</dt> <dd class="inline text-muted-foreground">{item.returns}</dd></div>
					{/if}
				</dl>
				{#if item.exampleRust && item.exampleJs}
					<Tabs.Root value="js" class="mt-3">
						<Tabs.List>
							<Tabs.Trigger value="js">JS</Tabs.Trigger>
							<Tabs.Trigger value="rust">Rust</Tabs.Trigger>
						</Tabs.List>
						<Tabs.Content value="js">
							<pre class="overflow-x-auto rounded bg-muted/40 p-2 text-xs"><code>{item.exampleJs}</code></pre>
						</Tabs.Content>
						<Tabs.Content value="rust">
							<pre class="overflow-x-auto rounded bg-muted/40 p-2 text-xs"><code>{item.exampleRust}</code></pre>
						</Tabs.Content>
					</Tabs.Root>
				{:else if item.exampleRust || item.exampleJs}
					<pre class="mt-3 overflow-x-auto rounded bg-muted/40 p-2 text-xs"><code>{item.exampleRust ?? item.exampleJs}</code></pre>
				{/if}
				{#if item.caveats && item.caveats.length > 0}
					<div class="mt-3">
						<h3 class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Caveats</h3>
						<ul class="mt-1 list-disc space-y-0.5 pl-5 text-sm text-muted-foreground">
							{#each item.caveats as caveat, i (i)}
								<li>{caveat}</li>
							{/each}
						</ul>
					</div>
				{/if}
			</section>
		{/each}
	</div>
{:else}
	<div class="prose prose-neutral mt-6 dark:prose-invert">
		<p>
			<em>Per-item reference for this module is in progress — see the rustdoc in
			<code>crates/gunmetal/src/{data.moduleRef.name}</code>.</em>
		</p>
	</div>
{/if}

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

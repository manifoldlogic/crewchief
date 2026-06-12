<script lang="ts">
	import { page } from '$app/state';
	import { demosForGrid, modules, orderedChapters } from '$lib/catalog';
	import { Input } from '$lib/components/ui/input';

	interface Entry {
		href: string;
		label: string;
		text: string;
		flagship?: boolean;
	}

	const guides: Entry[] = [
		{ href: '/gunmetal/quickstart', label: 'Quickstart', text: 'quickstart install setup get started cargo wasm npm' },
		{ href: '/gunmetal/troubleshooting', label: 'Troubleshooting', text: 'troubleshooting debug not syncing error symptom help' },
		{ href: '/gunmetal/glossary', label: 'Glossary', text: 'glossary terms acronym sea ham dam rad axe lex soul' }
	];

	const learn: Entry[] = orderedChapters.map((c) => ({
		href: `/gunmetal/learn/${c.slug}`,
		label: `${c.num}. ${c.label}`,
		text: [c.title, c.label, ...c.keywords].join(' ').toLowerCase()
	}));

	const demoEntries: Entry[] = demosForGrid().map((d) => ({
		href: `/gunmetal/demos/${d.slug}`,
		label: d.title,
		flagship: d.flagship,
		text: [d.title, d.pattern, d.capability, ...d.keywords, ...d.modules].join(' ').toLowerCase()
	}));

	const refEntries: Entry[] = modules.map((m) => ({
		href: `/gunmetal/reference/${m.name}`,
		label: m.name,
		text: [m.name, m.purpose, ...m.keywords].join(' ').toLowerCase()
	}));

	const groups = [
		{ title: 'Guides', entries: guides },
		{ title: 'Learn', entries: learn },
		{ title: 'Demos', entries: demoEntries },
		{ title: 'Reference', entries: refEntries }
	];

	let query = $state('');

	const visible = $derived(
		groups
			.map((g) => ({
				...g,
				entries: query.trim()
					? g.entries.filter((e) => e.text.includes(query.trim().toLowerCase()))
					: g.entries
			}))
			.filter((g) => g.entries.length > 0)
	);
</script>

<aside class="w-60 shrink-0 border-r" data-testid="gunmetal-sidebar">
	<div class="sticky top-14 max-h-[calc(100vh-3.5rem)] overflow-y-auto p-4">
		<Input
			type="search"
			placeholder="Filter docs…"
			bind:value={query}
			data-testid="sidebar-filter"
			class="mb-4 h-8"
		/>
		{#each visible as group (group.title)}
			<div class="mb-4">
				<h4 class="mb-1 px-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
					{group.title}
				</h4>
				<ul>
					{#each group.entries as entry (entry.href)}
						<li>
							<a
								href={entry.href}
								class="block rounded-md px-2 py-1 text-sm transition-colors {page.url.pathname ===
								entry.href
									? 'bg-secondary font-medium'
									: 'text-muted-foreground hover:text-foreground'}"
							>
								{entry.label}
								{#if entry.flagship}<span class="ml-1 rounded bg-primary px-1 text-[10px] text-primary-foreground">flagship</span>{/if}
							</a>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
		{#if visible.length === 0}
			<p class="px-2 text-sm text-muted-foreground">No matches.</p>
		{/if}
	</div>
</aside>

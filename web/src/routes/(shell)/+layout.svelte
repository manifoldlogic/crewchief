<script lang="ts">
	import { page } from '$app/state';
	import ThemeToggle from '$lib/components/shell/ThemeToggle.svelte';

	let { children } = $props();

	const tabs = [
		{ href: '/gunmetal', label: 'Gunmetal', testid: 'tab-gunmetal' },
		{ href: '/maproom', label: 'Maproom', testid: 'tab-maproom' },
		{ href: '/crewchief', label: 'CrewChief CLI', testid: 'tab-crewchief' }
	];

	const isActive = (href: string) =>
		page.url.pathname === href || page.url.pathname.startsWith(href + '/');
</script>

<div class="flex min-h-screen flex-col bg-background text-foreground">
	<header class="sticky top-0 z-40 border-b bg-background/95 backdrop-blur">
		<div class="mx-auto flex h-14 w-full max-w-7xl items-center gap-6 px-4">
			<a href="/" class="font-semibold tracking-tight" data-testid="home-link">crewchief docs</a>
			<nav class="flex items-center gap-1" data-testid="product-tabs">
				{#each tabs as tab (tab.href)}
					<a
						href={tab.href}
						data-testid={tab.testid}
						class="rounded-md px-3 py-1.5 text-sm transition-colors {isActive(tab.href)
							? 'bg-secondary font-medium text-secondary-foreground'
							: 'text-muted-foreground hover:text-foreground'}"
					>
						{tab.label}
					</a>
				{/each}
			</nav>
			<div class="ml-auto flex items-center gap-2">
				<a
					href="https://github.com/manifoldlogic/crewchief"
					class="text-sm text-muted-foreground hover:text-foreground"
					rel="external noreferrer"
					target="_blank"
				>
					GitHub
				</a>
				<ThemeToggle />
			</div>
		</div>
	</header>

	<main class="flex-1">
		{@render children()}
	</main>

	<footer class="border-t">
		<div
			class="mx-auto w-full max-w-7xl px-4 py-4 text-xs text-muted-foreground"
			data-testid="build-info"
		>
			gunmetal v{__GUNMETAL_VERSION__} · build {__BUILD_SHA__} · docs built from this repo
		</div>
	</footer>
</div>

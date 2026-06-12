<script lang="ts">
	import { toggleMode } from 'mode-watcher';
	import SunIcon from '@lucide/svelte/icons/sun';
	import MoonIcon from '@lucide/svelte/icons/moon';
	import { Button } from '$lib/components/ui/button';
</script>

<Button onclick={toggleMode} variant="ghost" size="icon" data-testid="theme-toggle" aria-label="Toggle dark mode">
	<SunIcon class="size-5 dark:hidden" />
	<MoonIcon class="hidden size-5 dark:block" />
</Button>

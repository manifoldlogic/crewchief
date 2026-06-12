<script lang="ts">
	import Sidebar from '$lib/components/shell/Sidebar.svelte';

	let { children } = $props();
</script>

<div class="mx-auto flex w-full max-w-7xl">
	<Sidebar />
	<div class="min-w-0 flex-1 px-6 py-8">
		{@render children()}
	</div>
</div>

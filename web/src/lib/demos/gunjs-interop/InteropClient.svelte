<script lang="ts">
	// gunjs-interop client: frame "a" boots real GUN.js, frame "b" boots
	// gunmetal wasm (?engine= decides at runtime). Both speak to the same
	// room soul through the same relay — the parity spec's acceptance
	// test, live in a browser.
	import { onMount } from 'svelte';
	import SharedInputClient from '$lib/demos/shared-input/SharedInputClient.svelte';
	import GunJsSharedInput from './GunJsSharedInput.svelte';
	import { parseClientParams } from '$lib/gun/client';

	let { slug = 'gunjs-interop' }: { slug?: string } = $props();

	let engine = $state<'gun' | 'gunmetal' | null>(null);

	onMount(() => {
		engine = parseClientParams(window.location).engine;
	});
</script>

{#if engine === 'gun'}
	<GunJsSharedInput {slug} />
{:else if engine === 'gunmetal'}
	<SharedInputClient {slug} />
{/if}

/**
 * Maps demo slugs to their client components (rendered by the bare
 * /client pages inside iframes). Only imported by the client route.
 */
import type { Component } from 'svelte';
import SharedInputClient from './shared-input/SharedInputClient.svelte';
import InteropClient from './gunjs-interop/InteropClient.svelte';

export const clientRegistry: Record<string, Component<{ slug?: string }>> = {
	'shared-input': SharedInputClient,
	'gunjs-interop': InteropClient
};

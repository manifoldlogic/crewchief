/**
 * Maps demo slugs to their client components (rendered by the bare
 * /client pages inside iframes). Only imported by the client route.
 */
import type { Component } from 'svelte';
import SharedInputClient from './shared-input/SharedInputClient.svelte';

export const clientRegistry: Record<string, Component> = {
	'shared-input': SharedInputClient
};

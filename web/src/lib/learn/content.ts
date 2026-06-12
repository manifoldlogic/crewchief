/**
 * Learn-chapter content components, keyed by chapter slug. Chapters not
 * yet written fall back to the stub in the [chapter] route.
 */
import type { Component } from 'svelte';
import WhyDecentralized from './chapters/WhyDecentralized.svelte';
import TheGraph from './chapters/TheGraph.svelte';
import Reactivity from './chapters/Reactivity.svelte';
import Collections from './chapters/Collections.svelte';
import Documents from './chapters/Documents.svelte';
import Sync from './chapters/Sync.svelte';

export const chapterContent: Record<string, Component> = {
	'why-decentralized': WhyDecentralized,
	'the-graph': TheGraph,
	reactivity: Reactivity,
	collections: Collections,
	documents: Documents,
	sync: Sync
};

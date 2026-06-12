import { error } from '@sveltejs/kit';
import { chaptersForDemo, demoBySlug, demos } from '$lib/catalog';
import type { EntryGenerator, PageLoad } from './$types';

export const entries: EntryGenerator = () => demos.map((d) => ({ slug: d.slug }));

export const load: PageLoad = ({ params }) => {
	const demo = demoBySlug.get(params.slug);
	if (!demo) error(404, `No such demo: ${params.slug}`);
	return { demo, chapters: chaptersForDemo(demo.slug) };
};

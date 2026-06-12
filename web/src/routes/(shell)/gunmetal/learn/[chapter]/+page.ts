import { error } from '@sveltejs/kit';
import { chapterBySlug, orderedChapters } from '$lib/catalog';
import type { EntryGenerator, PageLoad } from './$types';

export const entries: EntryGenerator = () =>
	orderedChapters.map((c) => ({ chapter: c.slug }));

export const load: PageLoad = ({ params }) => {
	const chapter = chapterBySlug.get(params.chapter);
	if (!chapter) error(404, `No such chapter: ${params.chapter}`);
	const prev = orderedChapters.find((c) => c.num === chapter.num - 1) ?? null;
	const next = orderedChapters.find((c) => c.num === chapter.num + 1) ?? null;
	return { chapter, prev, next, total: orderedChapters.length };
};

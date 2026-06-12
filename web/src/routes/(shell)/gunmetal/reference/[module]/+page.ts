import { error } from '@sveltejs/kit';
import { demosForModule, moduleByName, modules } from '$lib/catalog';
import type { EntryGenerator, PageLoad } from './$types';

export const entries: EntryGenerator = () => modules.map((m) => ({ module: m.name }));

export const load: PageLoad = ({ params }) => {
	const moduleRef = moduleByName.get(params.module);
	if (!moduleRef) error(404, `No such module: ${params.module}`);
	return { moduleRef, demos: demosForModule(moduleRef.name) };
};

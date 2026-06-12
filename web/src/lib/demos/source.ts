/**
 * Raw client sources for the §4(d) annotated-source view on demo pages.
 * Loaded lazily (runtime, client-side) so the source never bloats the
 * page bundle.
 */
const rawSources = import.meta.glob('./*/*.svelte', {
	query: '?raw',
	import: 'default'
});

export interface DemoSourceFile {
	name: string;
	code: string;
}

export async function demoSources(slug: string): Promise<DemoSourceFile[]> {
	const files: DemoSourceFile[] = [];
	for (const [path, loader] of Object.entries(rawSources)) {
		if (path.startsWith(`./${slug}/`)) {
			files.push({
				name: path.split('/').pop() ?? path,
				code: (await loader()) as string
			});
		}
	}
	return files.sort((a, b) => a.name.localeCompare(b.name));
}

/** Heuristic for the harness/API visual distinction: lines that talk to
 * the gunmetal API (gun./sea./cert./user. calls, wasm boot) vs. demo
 * harness (params, postMessage, readiness, markup). */
export function isApiLine(line: string): boolean {
	return /\b(gun|sea|cert|user)[?.!]*\.\w|bootGunmetal|WasmGun|WasmSEA|WasmUser|WasmCert/.test(
		line
	);
}

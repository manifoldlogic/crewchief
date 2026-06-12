/**
 * Production static server for the built docs site (spec §5.2).
 *
 * Serves `build/` with explicit resolution rules:
 *   exact file → path + '.html' → path + '/index.html' → 404
 * Query strings are stripped before resolution (iframe srcs carry params).
 * `/_app/immutable/*` gets immutable cache headers; HTML is never cached.
 * Bun.file infers MIME types, including application/wasm.
 *
 * Usage: bun server.ts [--port 4173]   (or PORT env)
 */

import { existsSync, statSync } from 'node:fs';
import { join, normalize } from 'node:path';

function isFile(path: string): boolean {
	try {
		return statSync(path).isFile();
	} catch {
		return false;
	}
}

const args = process.argv.slice(2);
const portFlag = args.indexOf('--port');
const port = Number(
	(portFlag !== -1 ? args[portFlag + 1] : undefined) ?? process.env.PORT ?? 4173
);

const root = join(import.meta.dir, 'build');

if (!existsSync(root)) {
	console.error(`No build/ directory at ${root} — run \`bun run build\` first.`);
	process.exit(1);
}

function resolve(pathname: string): string | null {
	// Decode and normalize; reject traversal out of the root.
	let decoded: string;
	try {
		decoded = decodeURIComponent(pathname);
	} catch {
		return null;
	}
	const normalized = normalize(decoded);
	if (normalized.includes('..')) return null;

	const base = join(root, normalized);
	const candidates =
		normalized.endsWith('/') || normalized === '/'
			? [join(base, 'index.html')]
			: [base, `${base}.html`, join(base, 'index.html')];

	for (const candidate of candidates) {
		if (isFile(candidate)) return candidate;
	}
	return null;
}

function cacheHeader(path: string): string {
	if (path.includes('/_app/immutable/')) return 'public, max-age=31536000, immutable';
	if (path.endsWith('.html')) return 'no-cache';
	return 'public, max-age=3600';
}

const server = Bun.serve({
	port,
	fetch(req) {
		const url = new URL(req.url); // .pathname strips the query string
		const path = resolve(url.pathname);
		if (!path) {
			return new Response('Not found', { status: 404 });
		}
		return new Response(Bun.file(path), {
			headers: { 'cache-control': cacheHeader(path) }
		});
	}
});

console.log(`docs site serving build/ at http://localhost:${server.port}`);

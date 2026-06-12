import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { mdsvex } from 'mdsvex';
import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Build provenance for the shell footer: which crate version and commit
// these docs were built from (spec §3.1).
function gunmetalVersion(): string {
	try {
		const cargo = readFileSync(new URL('../crates/gunmetal/Cargo.toml', import.meta.url), 'utf8');
		return /\bversion\s*=\s*"([^"]+)"/.exec(cargo)?.[1] ?? '0.0.0';
	} catch {
		return '0.0.0';
	}
}

function buildSha(): string {
	try {
		return execSync('git rev-parse --short HEAD', { cwd: import.meta.dirname }).toString().trim();
	} catch {
		return 'dev';
	}
}

export default defineConfig({
	define: {
		__GUNMETAL_VERSION__: JSON.stringify(gunmetalVersion()),
		__BUILD_SHA__: JSON.stringify(buildSha())
	},
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) => filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter(),
			prerender: {
				// The build is itself the dead-link check (spec §5.1).
				handleHttpError: 'fail'
			},
			preprocess: [mdsvex({ extensions: ['.svx', '.md'] })],
			extensions: ['.svelte', '.svx', '.md']
		})
	]
});

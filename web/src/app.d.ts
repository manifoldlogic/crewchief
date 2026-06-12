// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	/** Crate version baked in at build time (vite define). */
	const __GUNMETAL_VERSION__: string;
	/** Short git SHA baked in at build time (vite define). */
	const __BUILD_SHA__: string;
}

export {};

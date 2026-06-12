import { defineConfig } from '@playwright/test';

/**
 * E2E config (spec §8). webServer entries only exec prebuilt artifacts —
 * all builds happen in the `test:e2e` script BEFORE playwright runs.
 * The relay webServer entry joins in the demo phases.
 */
export default defineConfig({
	testDir: 'e2e',
	testMatch: '**/*.e2e.{ts,js}',
	use: {
		baseURL: 'http://localhost:4173'
	},
	webServer: [
		{
			command: 'bun server.ts --port 4173',
			url: 'http://localhost:4173/',
			reuseExistingServer: !process.env.CI
		}
	]
});

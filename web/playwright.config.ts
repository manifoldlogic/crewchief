import { defineConfig } from '@playwright/test';

/**
 * E2E config (spec §8). webServer entries only exec prebuilt artifacts —
 * all builds happen in the `test:e2e` script BEFORE playwright runs.
 * The relay webServer entry joins in the demo phases.
 */
export default defineConfig({
	testDir: 'e2e',
	testMatch: '**/*.e2e.{ts,js}',
	// Serial on purpose: each test page boots multiple wasm instances
	// (and the SEA demos run deliberately-slow PBKDF2); parallel workers
	// starve each other's readiness timeouts on shared-CPU runners. The
	// whole suite stays under ~2 minutes, and all tests share one relay.
	workers: 1,
	use: {
		baseURL: 'http://localhost:4173'
	},
	webServer: [
		{
			command: 'bun server.ts --port 4173',
			url: 'http://localhost:4173/',
			reuseExistingServer: !process.env.CI
		},
		{
			// Prebuilt by test:e2e (relay:build); fresh radata per run.
			command:
				'rm -rf /tmp/gunmetal-e2e-radata && ../target/debug/gunmetal-relay --port 8766 --file /tmp/gunmetal-e2e-radata',
			url: 'http://localhost:8766/health',
			reuseExistingServer: !process.env.CI
		}
	]
});

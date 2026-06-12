import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo } from './helpers';

test.describe('benchmark demo', () => {
	test('both engines run the workloads and report numbers', async ({ page }) => {
		// A measurement instrument, not a gate: we assert both engines
		// COMPLETE and report numeric rows — never who wins (shared-CPU
		// runners make magnitude assertions flaky by design).
		test.slow(); // PBKDF2 ×3 per engine, plus 6 other workloads each
		await gotoDemo(page, 'benchmark');
		const gunjs = frame(page, 'a');
		const gunmetal = frame(page, 'b');
		await awaitReady(gunjs);
		await awaitReady(gunmetal);

		await gunjs.getByTestId('bench-run').click();
		await gunmetal.getByTestId('bench-run').click();

		for (const engineFrame of [gunjs, gunmetal]) {
			await expect(engineFrame.locator('[data-bench-done="true"]')).toBeVisible({
				timeout: 60_000
			});
			const benchRows = engineFrame.getByTestId('bench-row');
			await expect(benchRows.first()).toBeVisible();
			expect(await benchRows.count()).toBeGreaterThanOrEqual(7);
			// every row carries a parseable millisecond figure
			for (const text of await benchRows.allInnerTexts()) {
				expect(text).toMatch(/[\d,.]+ ms/);
			}
		}
	});
});

import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo } from './helpers';

test.describe('benchmark demo', () => {
	test('both engines run the workloads and report numbers', async ({ page }) => {
		// A measurement instrument, not a gate: we assert both engines
		// COMPLETE and report numeric rows — never who wins (shared-CPU
		// runners make magnitude assertions flaky by design).
		// 5 passes × (PBKDF2 ×3 + 7 other workloads) per engine.
		test.setTimeout(240_000);
		await gotoDemo(page, 'benchmark');
		const gunjs = frame(page, 'a');
		const gunmetal = frame(page, 'b');
		await awaitReady(gunjs);
		await awaitReady(gunmetal);

		await gunjs.getByTestId('bench-run').click();
		await gunmetal.getByTestId('bench-run').click();

		for (const engineFrame of [gunjs, gunmetal]) {
			await expect(engineFrame.locator('[data-bench-done="true"]')).toBeVisible({
				timeout: 180_000
			});
			const benchRows = engineFrame.getByTestId('bench-row');
			await expect(benchRows.first()).toBeVisible();
			expect(await benchRows.count()).toBeGreaterThanOrEqual(7);
			// every row carries a parseable, finite millisecond figure
			for (const text of await benchRows.allInnerTexts()) {
				const m = text.match(/([\d,]+(?:\.\d+)?)\s*ms\b/);
				expect(m, `row missing a ms value: ${text}`).not.toBeNull();
				const value = Number(m![1].replace(/,/g, ''));
				expect(Number.isFinite(value), `unparseable ms value: ${text}`).toBe(true);
			}
		}
	});
});

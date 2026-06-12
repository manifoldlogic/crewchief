import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo, TEST_RELAY } from './helpers';

// The flagship: real GUN.js (vendored, frame "a") and gunmetal wasm
// (frame "b") on the same relay, editing the same soul. This recreates
// the gunmetal-parity.md wire-compatibility acceptance test live in a
// browser, in BOTH directions.
test.describe('gunjs-interop demo', () => {
	test('GUN.js and gunmetal sync the same input, both directions', async ({ page }) => {
		await gotoDemo(page, 'gunjs-interop');

		const gunjs = frame(page, 'a');
		const gunmetal = frame(page, 'b');
		await awaitReady(gunjs);
		await awaitReady(gunmetal);

		// GUN.js writes → gunmetal reads.
		await gunjs.getByTestId('shared-input').fill('written by GUN.js');
		await expect(gunmetal.getByTestId('shared-input')).toHaveValue('written by GUN.js', {
			timeout: 10_000
		});

		// gunmetal writes → GUN.js reads.
		await gunmetal.getByTestId('shared-input').fill('written by gunmetal');
		await expect(gunjs.getByTestId('shared-input')).toHaveValue('written by gunmetal', {
			timeout: 10_000
		});
	});

	test('landing page embeds the live flagship', async ({ page }) => {
		await page.goto(`/gunmetal?relay=${encodeURIComponent(TEST_RELAY)}`);
		const embed = page.getByTestId('flagship-embed');
		await expect(embed.getByTestId('frame-a')).toBeVisible();
		await expect(embed.getByTestId('frame-b')).toBeVisible();
	});
});

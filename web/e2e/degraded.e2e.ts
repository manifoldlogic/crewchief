import { expect, test } from '@playwright/test';

test.describe('relay-unreachable degraded state', () => {
	test('a client pointed at a dead relay shows instructions, not a spinner', async ({
		page
	}) => {
		// Port 9 (discard) — nothing listens there.
		const room = `e2e-degraded-${Math.random().toString(36).slice(2, 10)}`;
		await page.goto(
			`/gunmetal/demos/shared-input/client?room=${room}&frameId=solo&relay=${encodeURIComponent(
				'ws://localhost:9/gun'
			)}`
		);
		await expect(page.locator('body')).toHaveAttribute('data-ready', 'degraded', {
			timeout: 30_000
		});
		await expect(page.getByTestId('client-degraded')).toContainText('Relay unreachable');
		await expect(page.getByTestId('client-degraded')).toContainText('gunmetal-relay');
	});
});

import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo } from './helpers';

test.describe('wire-inspector demo', () => {
	test('shows the handshake, puts, gets, and acks crossing the wire', async ({ page }) => {
		await gotoDemo(page, 'wire-inspector');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		const logA = a.getByTestId('wire-log');
		const logB = b.getByTestId('wire-log');

		// The DAM handshake was captured (onWire registered pre-connect).
		await expect(logA.locator('li[data-kind="dam:?"]').first()).toBeVisible();
		// Boot fetch: an outgoing GET.
		await expect(logA.locator('li[data-dir="out"][data-kind="get"]').first()).toBeVisible();

		// A real write: A sends a put; the relay acks A; B receives a put.
		await a.getByTestId('shared-input').fill('watch the wire');
		await expect(
			logA.locator('li[data-dir="out"][data-kind="put"]').first()
		).toBeVisible({ timeout: 10_000 });
		await expect(logA.locator('li[data-kind="ack"]').first()).toBeVisible({
			timeout: 10_000
		});
		await expect(
			logB.locator('li[data-dir="in"][data-kind="put"]').first()
		).toBeVisible({ timeout: 10_000 });
		// And the app still works like a normal shared input.
		await expect(b.getByTestId('shared-input')).toHaveValue('watch the wire', {
			timeout: 10_000
		});
	});
});

import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo } from './helpers';

// The P2 tracer bullet: two isolated wasm clients (separate iframe JS
// realms) syncing one input through the real gunmetal-relay. If this is
// green, the whole stack works: wasm boot → WebSocket → DAM handshake →
// PUT/GET routing → subscription delivery.
test.describe('shared-input demo', () => {
	test('syncs typing across two sessions, both directions', async ({ page }) => {
		await gotoDemo(page, 'shared-input');

		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		await a.getByTestId('shared-input').fill('hello from A');
		await expect(b.getByTestId('shared-input')).toHaveValue('hello from A', {
			timeout: 10_000
		});

		await b.getByTestId('shared-input').fill('and back from B');
		await expect(a.getByTestId('shared-input')).toHaveValue('and back from B', {
			timeout: 10_000
		});
	});

	test('a late-joining client receives existing state', async ({ page, context }) => {
		const room = await gotoDemo(page, 'shared-input');

		const a = frame(page, 'a');
		await awaitReady(a);
		await a.getByTestId('shared-input').fill('persisted before join');

		// A brand-new page (new session) joining the same room must pull
		// the current state from the relay via GET.
		const late = await context.newPage();
		await late.goto(
			`/gunmetal/demos/shared-input/client?room=${room}&frameId=late&relay=${encodeURIComponent(
				'ws://localhost:8766/gun'
			)}`
		);
		await expect(late.locator('body')).toHaveAttribute('data-ready', 'true', {
			timeout: 20_000
		});
		await expect(late.getByTestId('shared-input')).toHaveValue('persisted before join', {
			timeout: 10_000
		});
	});
});

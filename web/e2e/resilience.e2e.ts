import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo, TEST_RELAY } from './helpers';

test.describe('conflict-lab demo', () => {
	test('divergent offline edits converge identically after reconnect', async ({ page }) => {
		await gotoDemo(page, 'conflict-lab');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		// Baseline syncs.
		await a.getByTestId('shared-input').fill('baseline');
		await expect(b.getByTestId('shared-input')).toHaveValue('baseline', { timeout: 10_000 });

		// Split brain.
		await page.getByTestId('stage-disconnect').click();
		await expect(a.locator('[data-relay-state]')).toHaveAttribute('data-relay-state', 'disconnected');
		await expect(b.locator('[data-relay-state]')).toHaveAttribute('data-relay-state', 'disconnected');

		await a.getByTestId('shared-input').fill('written by A while split');
		await b.getByTestId('shared-input').fill('written by B while split');
		// Divergence is real: neither side sees the other.
		await expect(a.getByTestId('shared-input')).toHaveValue('written by A while split');
		await expect(b.getByTestId('shared-input')).toHaveValue('written by B while split');

		// Heal: HAM picks ONE winner, deterministically, on both sides.
		await page.getByTestId('stage-reconnect').click();
		await expect(a.locator('[data-relay-state]')).toHaveAttribute('data-relay-state', 'connected', { timeout: 15_000 });
		await expect(b.locator('[data-relay-state]')).toHaveAttribute('data-relay-state', 'connected', { timeout: 15_000 });

		await expect(async () => {
			const va = await a.getByTestId('shared-input').inputValue();
			const vb = await b.getByTestId('shared-input').inputValue();
			expect(va).toBe(vb);
			expect(va).toMatch(/written by [AB] while split/);
		}).toPass({ timeout: 15_000 });
	});
});

test.describe('offline-first demo', () => {
	test('per-session persistence: no leakage while offline, sync on reconnect, survives relay loss', async ({
		page
	}) => {
		const room = await gotoDemo(page, 'offline-first');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);
		await expect(a.locator('[data-relay-state]')).toHaveAttribute('data-relay-state', 'connected', { timeout: 10_000 });

		// Offline edits stay local: same origin, but distinct IndexedDB
		// names per session — B must NOT see A's offline write.
		await page.getByTestId('stage-disconnect').click();
		await expect(a.locator('[data-relay-state]')).toHaveAttribute('data-relay-state', 'disconnected');
		await a.getByTestId('shared-input').fill('offline edit from A');
		await page.waitForTimeout(1500);
		await expect(b.getByTestId('shared-input')).not.toHaveValue('offline edit from A');

		// Reconnect → the edit propagates.
		await page.getByTestId('stage-reconnect').click();
		await expect(b.getByTestId('shared-input')).toHaveValue('offline edit from A', {
			timeout: 15_000
		});

		// Reload A pointing at an unreachable relay: the page still shows
		// the data — hydrated from ITS OWN IndexedDB, no network at all.
		await page.evaluate(
			([roomId]) => {
				const el = document.querySelector('[data-testid="frame-a"]') as HTMLIFrameElement;
				el.src = `/gunmetal/demos/offline-first/client?room=${roomId}&frameId=a&relay=${encodeURIComponent(
					'ws://localhost:9/gun'
				)}`;
			},
			[room]
		);
		await awaitReady(a);
		await expect(a.locator('[data-relay-state]')).toHaveAttribute('data-relay-state', 'disconnected');
		await expect(a.getByTestId('shared-input')).toHaveValue('offline edit from A', {
			timeout: 10_000
		});
	});
});

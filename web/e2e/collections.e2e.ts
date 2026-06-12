import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo } from './helpers';

test.describe('graph-explorer demo (single frame)', () => {
	test('put updates the inspector and the subscription log', async ({ page }) => {
		await gotoDemo(page, 'graph-explorer');
		const a = frame(page, 'a');
		await awaitReady(a);

		await a.getByTestId('ge-key').fill('name');
		await a.getByTestId('ge-value').fill('Ada Lovelace');
		await a.getByTestId('ge-put').click();

		await expect(a.getByTestId('ge-inspector')).toContainText('Ada Lovelace');
		await expect(a.getByTestId('ge-log')).toContainText('put people/mark.name');
	});
});

test.describe('chat-room demo', () => {
	test('messages appear in both sessions in order', async ({ page }) => {
		await gotoDemo(page, 'chat-room');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		await a.getByTestId('chat-input').fill('first message');
		await a.getByTestId('chat-send').click();
		await expect(b.getByTestId('chat-messages')).toContainText('first message', {
			timeout: 10_000
		});

		await b.getByTestId('chat-input').fill('the reply');
		await b.getByTestId('chat-send').click();
		await expect(a.getByTestId('chat-messages')).toContainText('the reply', {
			timeout: 10_000
		});

		// Ordering: uuid keys are time-sortable, so "first message"
		// renders before "the reply" in BOTH sessions.
		for (const f of [a, b]) {
			const text = await f.getByTestId('chat-messages').innerText();
			expect(text.indexOf('first message')).toBeLessThan(text.indexOf('the reply'));
		}
	});
});

test.describe('todo-list demo', () => {
	test('add, toggle, and unset sync across sessions', async ({ page }) => {
		await gotoDemo(page, 'todo-list');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		await a.getByTestId('todo-input').fill('buy milk');
		await a.getByTestId('todo-add').click();
		const itemInB = b.getByTestId('todo-items').locator('li', { hasText: 'buy milk' });
		await expect(itemInB).toBeVisible({ timeout: 10_000 });

		// Toggle in B → done state lands in A.
		await itemInB.getByTestId('todo-toggle').check();
		const itemInA = a.getByTestId('todo-items').locator('li', { hasText: 'buy milk' });
		await expect(itemInA).toHaveAttribute('data-done', 'true', { timeout: 10_000 });

		// Remove in A (unset) → disappears in B.
		await itemInA.getByTestId('todo-remove').click();
		await expect(itemInB).toHaveCount(0, { timeout: 10_000 });
	});
});

test.describe('presence demo', () => {
	test('both sessions see each other; closing one writes its bye marker', async ({ page }) => {
		await gotoDemo(page, 'presence');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		await expect(a.getByTestId('presence-roster').locator('[data-member="b"][data-online="true"]')).toBeVisible({ timeout: 10_000 });
		await expect(b.getByTestId('presence-roster').locator('[data-member="a"][data-online="true"]')).toBeVisible({ timeout: 10_000 });

		// Kill session B's iframe: its WebSocket closes and the RELAY
		// applies the registered bye() write — session A sees "left"
		// without waiting for heartbeats to go stale.
		await page.evaluate(() => {
			document.querySelector('[data-testid="frame-b"]')?.remove();
		});
		await expect(a.getByTestId('presence-roster').locator('[data-member="b"][data-left="true"]')).toBeVisible({ timeout: 15_000 });
	});
});

test.describe('profile-tree demo (single frame)', () => {
	test('load() assembles linked nodes; not() drives the empty state', async ({ page }) => {
		await gotoDemo(page, 'profile-tree');
		const a = frame(page, 'a');
		await awaitReady(a);

		await a.getByTestId('pt-city').fill('Cambridge');
		await a.getByTestId('pt-save').click();
		await a.getByTestId('pt-load').click();

		// The assembled document inlines the linked address node.
		await expect(a.getByTestId('pt-document')).toContainText('Cambridge', { timeout: 10_000 });
		await expect(a.getByTestId('pt-document')).toContainText('address');

		// Absent soul → not() fires → empty state.
		await a.getByTestId('pt-check').click();
		await expect(a.getByTestId('pt-empty-state')).toBeVisible({ timeout: 10_000 });
	});
});

import { expect, test } from '@playwright/test';

test.describe('learn chapters', () => {
	test('chapter 0 defines the jargon before anything uses it', async ({ page }) => {
		await page.goto('/gunmetal/learn/why-decentralized');
		const body = page.locator('main');
		await expect(body).toContainText('soul');
		await expect(body).toContainText('GUN.js');
		await expect(body).toContainText('graph');
	});

	test('chapter 5 answers the why-a-server question and shows one frame', async ({ page }) => {
		await page.goto('/gunmetal/learn/sync');
		const body = page.locator('main');
		await expect(body).toContainText('Why is there a server in a decentralized system?');
		await expect(body).toContainText('not an authority');
		await expect(body).toContainText('"put"');
	});

	test('pre-sync chapters embed single-frame demos that boot without a relay', async ({
		page
	}) => {
		await page.goto('/gunmetal/learn/collections');
		// First embed: todo-list, single frame, no relay needed — the
		// readiness contract must still resolve.
		const firstFrame = page
			.getByTestId('chapter-embeds')
			.locator('iframe')
			.first();
		await expect(firstFrame).toBeVisible();
		const frame = page.frameLocator('[data-testid="chapter-embeds"] iframe >> nth=0');
		await expect(frame.locator('body')).toHaveAttribute('data-ready', 'true', {
			timeout: 20_000
		});
		// And the forward hook to the security chapters is present.
		await expect(page.locator('main')).toContainText('lock things down');
	});

	test('chapter footers chain prev/next through the whole path', async ({ page }) => {
		await page.goto('/gunmetal/learn/the-graph');
		await page.getByTestId('chapter-next').click();
		await expect(page).toHaveURL(/reactivity/);
		await page.getByTestId('chapter-prev').click();
		await expect(page).toHaveURL(/the-graph/);
	});
});

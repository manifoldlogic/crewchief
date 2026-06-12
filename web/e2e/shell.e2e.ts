import { expect, test } from '@playwright/test';
import { allRoutes } from '../src/lib/catalog';

test.describe('shell', () => {
	test('product tabs navigate between sections', async ({ page }) => {
		await page.goto('/');
		await expect(page.getByTestId('product-cards')).toBeVisible();

		await page.getByTestId('tab-gunmetal').click();
		await expect(page).toHaveURL(/\/gunmetal$/);
		await expect(page.getByRole('heading', { name: 'Gunmetal', exact: true })).toBeVisible();

		await page.getByTestId('tab-maproom').click();
		await expect(page).toHaveURL(/\/maproom$/);
		await expect(page.getByRole('heading', { name: 'Maproom' })).toBeVisible();

		await page.getByTestId('tab-crewchief').click();
		await expect(page).toHaveURL(/\/crewchief$/);
	});

	test('dark mode toggles and persists across reload', async ({ page }) => {
		await page.goto('/gunmetal');
		const html = page.locator('html');
		const initiallyDark = await html.evaluate((el) => el.classList.contains('dark'));

		await page.getByTestId('theme-toggle').click();
		await expect(html).toHaveClass(initiallyDark ? /^(?!.*dark).*$/ : /dark/);

		await page.reload();
		const afterReload = await html.evaluate((el) => el.classList.contains('dark'));
		expect(afterReload).toBe(!initiallyDark);
	});

	test('sidebar filter finds pages by API symbol', async ({ page }) => {
		await page.goto('/gunmetal');
		const sidebar = page.getByTestId('gunmetal-sidebar');

		// "unset" is an API symbol — it must surface the extended reference
		// page and the todo-list demo, and hide unrelated entries.
		await page.getByTestId('sidebar-filter').fill('unset');
		await expect(sidebar.locator('a[href="/gunmetal/reference/extended"]')).toBeVisible();
		await expect(sidebar.locator('a[href="/gunmetal/demos/todo-list"]')).toBeVisible();
		await expect(sidebar.locator('a[href="/gunmetal/reference/relay"]')).toHaveCount(0);

		// "authPair" must hit the user module.
		await page.getByTestId('sidebar-filter').fill('authPair');
		await expect(sidebar.locator('a[href="/gunmetal/reference/user"]')).toBeVisible();
	});

	test('landing page carries the required §3.4 blocks', async ({ page }) => {
		await page.goto('/gunmetal');
		await expect(page.getByTestId('cta-learn')).toBeVisible();
		await expect(page.getByTestId('cta-quickstart')).toBeVisible();
		await expect(page.getByTestId('flagship-embed')).toBeVisible();
		await expect(page.getByTestId('capability-grid')).toBeVisible();
		// All 12 capabilities present in the grid.
		await expect(page.getByTestId('capability-grid').locator('[data-capability]')).toHaveCount(12);
		await expect(page.getByTestId('vs-gunjs')).toBeVisible();
		await expect(page.getByTestId('status-limits')).toBeVisible();
		await expect(page.getByTestId('quickstart-teaser')).toBeVisible();
	});

	test('demo grid pins the flagship first', async ({ page }) => {
		await page.goto('/gunmetal/demos');
		const first = page.getByTestId('demo-grid').locator('a').first();
		await expect(first).toHaveAttribute('data-demo', 'gunjs-interop');
		await expect(first.getByTestId('flagship-badge')).toBeVisible();
	});

	test('demo pages carry the full §4 format: why, snippets, gotchas', async ({ page }) => {
		// Spot-check two demos with different shapes; the content map is
		// keyed by slug, so coverage of all 14 is asserted statically below.
		for (const slug of ['shared-input', 'doc-permissions']) {
			await page.goto(`/gunmetal/demos/${slug}`);
			await expect(page.getByTestId('demo-why')).toBeVisible();
			await expect(page.getByTestId('demo-snippets').locator('pre').first()).toBeVisible();
			await expect(page.getByTestId('demo-gotchas').locator('li').first()).toBeVisible();
		}
	});

	test('every implemented demo has page content (why/snippets/gotchas)', async ({ request }) => {
		// The content map must cover every demo in the manifest — a demo
		// without its why/snippets/gotchas sections fails §4 acceptance.
		const { demos } = await import('../src/lib/catalog');
		const { demoContent } = await import('../src/lib/demos/content');
		for (const demo of demos) {
			expect(demoContent[demo.slug], `content for ${demo.slug}`).toBeTruthy();
			expect(demoContent[demo.slug].snippets.length, `snippets for ${demo.slug}`).toBeGreaterThan(0);
			expect(demoContent[demo.slug].gotchas.length, `gotchas for ${demo.slug}`).toBeGreaterThan(0);
		}
		void request;
	});

	test('build provenance footer is present', async ({ page }) => {
		await page.goto('/gunmetal');
		await expect(page.getByTestId('build-info')).toContainText(/gunmetal v\d+\.\d+\.\d+/);
	});

	test('every manifest route resolves (link integrity)', async ({ request }) => {
		for (const route of allRoutes()) {
			const response = await request.get(route);
			expect(response.status(), `route ${route}`).toBe(200);
		}
	});
});

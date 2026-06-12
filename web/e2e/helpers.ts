import { expect, type FrameLocator, type Page } from '@playwright/test';

export const TEST_RELAY = 'ws://localhost:8766/gun';

/** Open a demo page against the test relay with a unique room. */
export async function gotoDemo(page: Page, slug: string): Promise<string> {
	const room = `e2e-${Math.random().toString(36).slice(2, 10)}`;
	await page.goto(
		`/gunmetal/demos/${slug}?room=${room}&relay=${encodeURIComponent(TEST_RELAY)}`
	);
	return room;
}

export function frame(page: Page, id: string): FrameLocator {
	return page.frameLocator(`[data-testid="frame-${id}"]`);
}

/** The §5.5 readiness contract: never interact before this resolves. */
export async function awaitReady(frameLocator: FrameLocator): Promise<void> {
	await expect(frameLocator.locator('body')).toHaveAttribute('data-ready', 'true', {
		timeout: 20_000
	});
}

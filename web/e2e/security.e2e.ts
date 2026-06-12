import { expect, test } from '@playwright/test';
import { awaitReady, frame, gotoDemo, TEST_RELAY } from './helpers';

test.describe('login demo', () => {
	test('signup, logout, wrong password fails, login, session restores on reload', async ({
		page
	}) => {
		const room = await gotoDemo(page, 'login');
		const a = frame(page, 'a');
		await awaitReady(a);

		await a.getByTestId('login-password').fill('correct horse battery');
		await a.getByTestId('login-signup').click();
		await expect(a.getByTestId('login-status')).toContainText('Signed in as', {
			timeout: 15_000
		});

		await a.getByTestId('login-logout').click();
		await expect(a.getByTestId('login-status')).toContainText('Signed out');

		await a.getByTestId('login-password').fill('wrong password');
		await a.getByTestId('login-login').click();
		await expect(a.getByTestId('login-error')).toBeVisible({ timeout: 15_000 });

		await a.getByTestId('login-password').fill('correct horse battery');
		await a.getByTestId('login-login').click();
		await expect(a.getByTestId('login-status')).toContainText('Signed in as', {
			timeout: 15_000
		});

		// Reload the client: the saved pair restores the session with no
		// password.
		await page.evaluate(
			([roomId, relay]) => {
				const el = document.querySelector('[data-testid="frame-a"]') as HTMLIFrameElement;
				el.src = `/gunmetal/demos/login/client?room=${roomId}&frameId=a&relay=${encodeURIComponent(relay)}`;
			},
			[room, TEST_RELAY]
		);
		await awaitReady(a);
		await expect(a.getByTestId('login-status')).toContainText('Signed in as');
		await expect(a.getByTestId('login-restored')).toBeVisible();
	});
});

test.describe('private-notes demo', () => {
	test('graph stores ciphertext; same passphrase decrypts on the other session', async ({
		page
	}) => {
		await gotoDemo(page, 'private-notes');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		await a.getByTestId('pn-pass').fill('open sesame');
		await a.getByTestId('pn-unlock').click();
		await a.getByTestId('pn-note').fill('the secret plan');
		await a.getByTestId('pn-add').click();

		// Both sessions hold only ciphertext in the graph.
		await expect(a.getByTestId('pn-raw').locator('li')).toHaveCount(1, { timeout: 10_000 });
		await expect(a.getByTestId('pn-raw')).not.toContainText('the secret plan');
		await expect(b.getByTestId('pn-raw').locator('li')).toHaveCount(1, { timeout: 10_000 });
		await expect(b.getByTestId('pn-raw')).not.toContainText('the secret plan');

		// The right passphrase decrypts; the note reads in session B.
		await b.getByTestId('pn-pass').fill('open sesame');
		await b.getByTestId('pn-unlock').click();
		await expect(b.getByTestId('pn-notes')).toContainText('the secret plan', {
			timeout: 10_000
		});
	});
});

test.describe('secret-handshake demo', () => {
	test('ECDH-encrypted DMs flow both directions', async ({ page }) => {
		await gotoDemo(page, 'secret-handshake');
		const a = frame(page, 'a');
		const b = frame(page, 'b');
		await awaitReady(a);
		await awaitReady(b);

		await expect(a.getByTestId('sh-status')).toContainText('secure channel ready', {
			timeout: 15_000
		});
		await expect(b.getByTestId('sh-status')).toContainText('secure channel ready', {
			timeout: 15_000
		});

		await a.getByTestId('sh-input').fill('psst — over here');
		await a.getByTestId('sh-send').click();
		await expect(b.getByTestId('sh-messages')).toContainText('psst — over here', {
			timeout: 10_000
		});

		await b.getByTestId('sh-input').fill('received loud and clear');
		await b.getByTestId('sh-send').click();
		await expect(a.getByTestId('sh-messages')).toContainText('received loud and clear', {
			timeout: 10_000
		});
	});
});

test.describe('doc-permissions demo', () => {
	test('uncertified writes are rejected by readers; certified writes are accepted', async ({
		page
	}) => {
		await gotoDemo(page, 'doc-permissions');
		const owner = frame(page, 'a');
		const guest = frame(page, 'b');
		await awaitReady(owner);
		await awaitReady(guest);

		// Guest writes BEFORE any grant: readers must reject it.
		await guest.getByTestId('dp-input').fill('sneaky early write');
		await guest.getByTestId('dp-write').click();
		const rejected = owner
			.getByTestId('dp-entries')
			.locator('li[data-status="rejected"]', { hasText: 'sneaky early write' });
		await expect(rejected).toBeVisible({ timeout: 10_000 });

		// Owner grants; guest's cert arrives; a new write is accepted.
		await owner.getByTestId('dp-grant').click();
		await expect(guest.getByTestId('dp-cert-status')).toContainText('You hold a write certificate', {
			timeout: 10_000
		});
		await guest.getByTestId('dp-input').fill('hello with permission');
		await guest.getByTestId('dp-write').click();
		const accepted = owner
			.getByTestId('dp-entries')
			.locator('li[data-status="accepted"]', { hasText: 'hello with permission' });
		await expect(accepted).toBeVisible({ timeout: 10_000 });
	});
});

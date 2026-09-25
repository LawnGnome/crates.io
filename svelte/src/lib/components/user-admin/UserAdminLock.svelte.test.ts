import { http, HttpResponse } from 'msw';
import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';

import { NotificationsState } from '$lib/notifications.svelte';
import { loggedInSession, renderWithSession } from '$lib/utils/testing/session';
import { test } from '../../../test/msw';
import { ADMIN_USER, USER } from './test-data.svelte';
import UserAdminLock from './UserAdminLock.svelte';

describe('UserAdminLock', () => {
  it('disallows a lock if sudo is disabled', async () => {
    let session = loggedInSession(ADMIN_USER);

    await renderWithSession(UserAdminLock, { user: USER }, session);

    await page.getByCSS('[data-test-reason]').fill('Test');

    let button = page.getByCSS('[data-test-lock]');
    expect(button.element()).toBeDisabled();
  });

  test('handles the conflict from the server if the user is already locked', async ({ worker }) => {
    let session = loggedInSession(ADMIN_USER);
    session.setSudo(60 * 60 * 1000);

    let notifications = new NotificationsState();

    worker.use(
      http.put('/api/v1/users/:user/lock', async ({ params, request }) => {
        expect(params.user).toEqual(USER.login);

        let { reason, until } = (await request.json()) as { reason: string; until?: string | null };
        expect(reason).toEqual('Test');
        expect(until).toBeNullable();

        return HttpResponse.text('user is already locked', { status: 409 });
      }),
    );

    await renderWithSession(UserAdminLock, { user: USER }, session, notifications);

    await page.getByCSS('[data-test-reason]').fill('Test');

    let button = page.getByCSS('[data-test-lock]');
    expect(button.element()).toBeEnabled();
    await button.click();

    // Wait for the notification to be populated, which should happen at the
    // same time the button is re-enabled.
    await expect.element(button).toBeEnabled();
    expect(notifications.content).toHaveLength(1);
  });

  test('handles successful responses', async ({ worker }) => {
    let session = loggedInSession(ADMIN_USER);
    session.setSudo(60 * 60 * 1000);

    let notifications = new NotificationsState();

    worker.use(
      http.put('/api/v1/users/:user/lock', async ({ params, request }) => {
        expect(params.user).toEqual(USER.login);

        let { reason, until } = (await request.json()) as { reason: string; until?: string | null };
        expect(reason).toEqual('Test');
        expect(until).not.toBeNullable();

        // until will be ~30 days in the future, but obviously not exactly 30
        // days in the future, since we don't control how long test execution
        // takes. We'll accept anything from 29-31.
        let delta = (new Date(until as string).getTime() - Date.now()) / 1000 / 60 / 60 / 24;
        expect(delta).toBeGreaterThanOrEqual(29);
        expect(delta).toBeLessThanOrEqual(31);

        return HttpResponse.text('', { status: 204 });
      }),
    );

    await renderWithSession(UserAdminLock, { user: USER }, session, notifications);

    await page.getByCSS('[data-test-reason]').fill('Test');

    // We'll set an expiry as well from the dropdown.
    await page.getByCSS('[data-test-expiry]').selectOptions('30');

    let button = page.getByCSS('[data-test-lock]');
    expect(button.element()).toBeEnabled();
    await button.click();

    // Wait for the button to be re-enabled and then assert that no
    // notifications were created.
    await expect.element(button).toBeEnabled();
    expect(notifications.content).toHaveLength(0);
  });
});

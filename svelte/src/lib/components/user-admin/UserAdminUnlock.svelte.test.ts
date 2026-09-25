import { http, HttpResponse } from 'msw';
import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';

import { NotificationsState } from '$lib/notifications.svelte';
import { loggedInSession, renderWithSession } from '$lib/utils/testing/session';
import { test } from '../../../test/msw';
import { ADMIN_USER, USER } from './test-data.svelte';
import UserAdminUnlock from './UserAdminUnlock.svelte';

describe('UserAdminUnlock', () => {
  it('disallows an unlock if sudo is disabled', async () => {
    let session = loggedInSession(ADMIN_USER);

    await renderWithSession(UserAdminUnlock, { user: USER }, session);

    let button = page.getByCSS('[data-test-unlock]');
    expect(button.element()).toBeDisabled();
  });

  test('handles the conflict from the server if the user is already unlocked', async ({ worker }) => {
    let session = loggedInSession(ADMIN_USER);
    session.setSudo(60 * 60 * 1000);

    let notifications = new NotificationsState();

    worker.use(
      http.delete('/api/v1/users/:user/lock', ({ params }) => {
        expect(params.user).toEqual(USER.login);

        return HttpResponse.text('user is not currently locked', { status: 409 });
      }),
    );

    await renderWithSession(UserAdminUnlock, { user: USER }, session, notifications);

    let button = page.getByCSS('[data-test-unlock]');
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
      http.delete('/api/v1/users/:user/lock', ({ params }) => {
        expect(params.user).toEqual(USER.login);

        return HttpResponse.text('', { status: 204 });
      }),
    );

    await renderWithSession(UserAdminUnlock, { user: USER }, session, notifications);

    let button = page.getByCSS('[data-test-unlock]');
    expect(button.element()).toBeEnabled();
    await button.click();

    // Wait for the button to be re-enabled and then assert that no
    // notifications were created.
    await expect.element(button).toBeEnabled();
    expect(notifications.content).toHaveLength(0);
  });
});

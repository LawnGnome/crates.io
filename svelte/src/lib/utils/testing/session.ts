import type { NotificationsState } from '$lib/notifications.svelte';
import type { AuthenticatedUser } from '$lib/utils/session.svelte';
import type { Component } from 'svelte';

import { createClient } from '@crates-io/api-client';
import { render } from 'vitest-browser-svelte';

import { SessionState } from '$lib/utils/session.svelte';
import SessionHelper from './SessionHelper.svelte';

/**
 * Helper to render the given component within a session context.
 *
 * If the session parameter is omitted, a default session will be created (that
 * is, an anonymous user).
 */
export function renderWithSession<Props extends Record<string, unknown>>(
  component: Component<Props>,
  props: Props,
  session?: SessionState,
  notifications?: NotificationsState,
) {
  return render(
    SessionHelper as Component<{
      session?: SessionState;
      notifications?: NotificationsState;
      component: Component<Props>;
      props: Props;
    }>,
    { props: { session, notifications, component, props } },
  );
}

/**
 * Creates a logged in session state with the given user.
 */
export function loggedInSession(currentUser: AuthenticatedUser) {
  let session = new SessionState(createClient());
  session.setUser(currentUser);

  return session;
}

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

import { isLoggedIn } from '$lib/utils/session.svelte';

export async function load({ fetch, params, parent }) {
  let client = createClient({ fetch });

  let { user, linked_accounts: linkedAccounts } = await loadUser(client, params.user_id);

  // We gate the `parent()` call behind `isLoggedIn()` to avoid the overhead of
  // waiting for the `/api/v1/me` request for unauthenticated users.
  //
  // NOTE: `isLoggedIn()` reads from localStorage, which is only
  // available in the browser. This will need to be revisited if SSR
  // is implemented in the future.
  let currentUser;
  if (isLoggedIn()) {
    let { userPromise } = await parent();
    currentUser = await userPromise;
  }

  let lock = currentUser?.is_admin ? (await loadLock(client, params.user_id)).lock : undefined;

  return { user, linkedAccounts, currentUser, lock };
}

function loadUserError(login: string, status: number): never {
  if (status === 404) {
    error(404, { message: `${login}: User not found` });
  } else {
    error(status, { message: `${login}: Failed to load user data`, tryAgain: true });
  }
}

async function loadUser(client: ReturnType<typeof createClient>, login: string) {
  let response;
  try {
    response = await client.GET('/api/v1/users/{user}', {
      params: {
        path: { user: login },
        query: { include: 'linked_accounts' },
      },
    });
  } catch {
    // Network errors are treated as `504 Gateway Timeout`
    loadUserError(login, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadUserError(login, status);
  }

  return response.data;
}

async function loadLock(client: ReturnType<typeof createClient>, login: string) {
  let response;
  try {
    response = await client.GET('/api/v1/users/{user}/lock', {
      params: {
        path: { user: login },
      },
    });
  } catch {
    // Network errors are treated as `504 Gateway Timeout`
    loadUserError(login, 504);
  }

  let status = response.response.status;
  if (response.error) {
    loadUserError(login, status);
  }

  return response.data;
}

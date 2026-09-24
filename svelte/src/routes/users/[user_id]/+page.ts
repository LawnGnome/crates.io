import type { paths } from '@crates-io/api-client';

import { createClient } from '@crates-io/api-client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params, parent, url }) {
  let { user, linkedAccounts, currentUser, lock } = await parent();

  let client = createClient({ fetch });

  let pageStr = url.searchParams.get('page') ?? '1';
  let page = parseInt(pageStr, 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'alpha';

  // Check if the logged-in user is viewing their own profile, so that
  // yanked crates are included in the results.
  let isOwnProfile = currentUser?.id === user.id;

  let cratesResponse = await loadCrates(client, params.user_id, {
    user_id: user.id,
    page,
    per_page: perPage,
    sort,
    include_yanked: isOwnProfile ? 'yes' : 'n',
  });

  return { user, linkedAccounts, lock, cratesResponse, page, perPage, sort };
}

function loadUserError(login: string, status: number): never {
  if (status === 404) {
    error(404, { message: `${login}: User not found` });
  } else {
    error(status, { message: `${login}: Failed to load user data`, tryAgain: true });
  }
}

async function loadCrates(
  client: ReturnType<typeof createClient>,
  login: string,
  query: paths['/api/v1/crates']['get']['parameters']['query'],
) {
  let response;
  try {
    response = await client.GET('/api/v1/crates', { params: { query } });
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

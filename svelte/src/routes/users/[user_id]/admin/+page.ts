import { error } from '@sveltejs/kit';

export async function load({ parent }) {
  let { userPromise } = await parent();
  let currentUser = await userPromise;
  if (!currentUser?.is_admin) {
    error(403, { adminNeeded: true, message: 'You must be an admin to access this page' });
  }

  return await parent();
}

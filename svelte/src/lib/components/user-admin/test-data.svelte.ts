import type { AuthenticatedUser } from '$lib/utils/session.svelte';
import type { ComponentProps } from 'svelte';

import { SvelteDate } from 'svelte/reactivity';

import UserAdminMetadata from './UserAdminMetadata.svelte';

type Lock = ComponentProps<typeof UserAdminMetadata>['lock'];
type User = ComponentProps<typeof UserAdminMetadata>['user'];

export const ADMIN_USER: AuthenticatedUser = {
  id: 42,
  created_at: null,
  login: 'admin',
  name: 'Ad Min',
  avatar: 'https://avatars.githubusercontent.com/u/1234567?v=4',
  email: 'admin@example.com',
  email_verified: true,
  email_verification_sent: true,
  is_admin: true,
  publish_notifications: true,
  url: 'https://github.com/admin',
};

export const USER: User = {
  avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
  created_at: new SvelteDate(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
  github_username_matches: true,
  id: 1,
  login: 'janedoe',
  name: 'Jane Doe',
  url: 'https://github.com/janedoe',
};

export const INDEFINITE_LOCK: Lock = {
  reason: 'Locked indefinitely',
};

export const DEFINITE_LOCK: Lock = {
  reason: 'Locked definitely',
  until: new SvelteDate(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(),
};

import type { AuthenticatedUser } from '$lib/utils/session.svelte';

import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';

import { loggedInSession, renderWithSession } from '../utils/testing/session';
import UserPageHeader from './UserPageHeader.svelte';

const USER = {
  avatar: null,
  github_username_matches: true,
  login: 'crates-user',
  name: 'Crates User',
};

const LINKED_ACCOUNTS = [
  {
    account_id: '1',
    avatar: null,
    login: 'github-user',
    provider: 'github' as const,
  },
  {
    account_id: '2',
    avatar: null,
    login: 'crates-user',
    provider: 'github' as const,
  },
];

const AUTHENTICATED_USER: AuthenticatedUser = {
  created_at: null,
  email: 'user@example.com',
  email_verification_sent: true,
  email_verified: true,
  id: 1,
  is_admin: false,
  publish_notifications: true,
  url: 'https://github.com/github-user',
  ...USER,
};

describe('UserPageHeader', () => {
  it('renders every linked account', async () => {
    await renderWithSession(UserPageHeader, { user: USER, linkedAccounts: LINKED_ACCOUNTS });

    let chips = page.getByCSS('[data-test-account-chip]');
    expect(chips.elements()).toHaveLength(2);
    expect(chips.elements().map(chip => chip.getAttribute('href'))).toEqual([
      'https://github.com/github-user',
      'https://github.com/crates-user',
    ]);

    // As we didn't provide a user, no user tabs should appear.
    expect(page.getByCSS('[data-test-user-tabs]').elements()).toHaveLength(0);
  });

  it('does not mark GitHub accounts when one matches', async () => {
    await renderWithSession(UserPageHeader, { user: USER, linkedAccounts: LINKED_ACCOUNTS });

    expect(page.getByCSS('[data-test-mismatch-marker]').elements()).toHaveLength(0);

    // As we didn't provide a user, no user tabs should appear.
    expect(page.getByCSS('[data-test-user-tabs]').elements()).toHaveLength(0);
  });

  it('marks every GitHub account when none match', async () => {
    await renderWithSession(UserPageHeader, {
      user: { ...USER, github_username_matches: false },
      linkedAccounts: LINKED_ACCOUNTS,
    });

    expect(page.getByCSS('[data-test-mismatch-marker]').elements()).toHaveLength(2);

    // As we didn't provide a user, no user tabs should appear.
    expect(page.getByCSS('[data-test-user-tabs]').elements()).toHaveLength(0);
  });

  it('omits the account row when there are no linked accounts', async () => {
    await renderWithSession(UserPageHeader, { user: USER, linkedAccounts: [] });

    expect(page.getByCSS('.accounts').elements()).toHaveLength(0);

    // As we didn't provide a user, no user tabs should appear.
    expect(page.getByCSS('[data-test-user-tabs]').elements()).toHaveLength(0);
  });

  it('omits the user tabs when the user is signed in but not an admin', async () => {
    let session = loggedInSession(AUTHENTICATED_USER);
    await renderWithSession(UserPageHeader, { user: USER, linkedAccounts: [] }, session);

    // As the user is not an admin, no user tabs should appear.
    expect(page.getByCSS('[data-test-user-tabs]').elements()).toHaveLength(0);
  });

  it('shows the user tabs when the user is an admin', async () => {
    let session = loggedInSession({
      ...AUTHENTICATED_USER,
      is_admin: true,
    });
    await renderWithSession(UserPageHeader, { user: USER, linkedAccounts: [] }, session);

    // As the user is an admin, the user tabs should appear.
    expect(page.getByCSS('[data-test-user-tabs]').elements()).toHaveLength(1);
  });
});

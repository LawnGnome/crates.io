<!--
  @component
  Renders the public user identity header.
-->
<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';
  import { formatDistanceToNow } from 'date-fns';

  import * as NavTabs from '$lib/components/nav-tabs';
  import { getSession } from '$lib/utils/session.svelte';
  import AccountChip from './AccountChip.svelte';
  import Icon from './Icon.svelte';
  import PageHeader from './PageHeader.svelte';
  import Tooltip from './Tooltip.svelte';
  import UserAvatar from './UserAvatar.svelte';

  type LinkedAccount = Pick<components['schemas']['LinkedAccount'], 'account_id' | 'login' | 'provider'>;

  interface UserPageHeaderUser {
    /** The user's avatar URL, if available. */
    avatar?: string | null;

    /** The username displayed in the page heading. */
    login: string;

    /** Whether a linked GitHub username exactly matches the crates.io username. */
    github_username_matches: boolean;

    /** The user's optional display name. */
    name?: string | null;
  }

  export interface UserPageHeaderUserLock {
    /** The reason the user is locked. */
    reason: string;

    /**
     * When the user is locked until. If null, then the user is locked
     * indefinitely.
     */
    until?: string | null;
  }

  interface Props {
    /** The public user identity displayed in the header. */
    user: UserPageHeaderUser;

    /** The external accounts linked to the user. */
    linkedAccounts: LinkedAccount[];

    /** The lock status of the user. */
    lock?: UserPageHeaderUserLock | null;
  }

  let { user, linkedAccounts, lock }: Props = $props();

  function buildUrl(account: LinkedAccount): string {
    switch (account.provider) {
      case 'github':
        return `https://github.com/${account.login}`;
    }
  }

  let lockTooltip = $derived.by(() => {
    if (lock) {
      if (lock.until) {
        return `Locked for ${formatDistanceToNow(lock.until)}: ${lock.reason}`;
      } else {
        return `Locked indefinitely: ${lock.reason}`;
      }
    }
  });

  let session = getSession();

  let cratesHref = $derived(resolve('/users/[user_id]', { user_id: user.login }));
  let metadataHref = $derived(resolve('/users/[user_id]/admin', { user_id: user.login }));
</script>

<PageHeader data-test-heading>
  <div class="layout">
    <UserAvatar
      user={{ avatar: user.avatar, kind: 'user', login: user.login, name: user.name }}
      size="medium"
      class="user-page-avatar"
      data-test-avatar
    />
    <div class="identity">
      <h1 data-test-username>
        {user.login}
        {#if lockTooltip}
          <div>
            <Icon class="i-mdi:lock" />
            <Tooltip text={lockTooltip} />
          </div>
        {/if}
      </h1>
      {#if user.name}
        <div class="display-name" data-test-display-name>{user.name}</div>
      {/if}
      {#if linkedAccounts.length !== 0}
        <div class="accounts">
          {#each linkedAccounts as account (account.account_id)}
            <AccountChip
              provider={account.provider}
              handle={account.login}
              href={buildUrl(account)}
              mismatched={account.provider === 'github' && !user.github_username_matches}
            />
          {/each}
        </div>
      {/if}
    </div>
  </div>
</PageHeader>

{#if session.currentUser?.is_admin}
  <NavTabs.Root aria-label="{user.login} user subpages" class="mb-s" data-test-user-tabs>
    <NavTabs.Tab href={cratesHref} data-test-crates-tab>Crates</NavTabs.Tab>
    <NavTabs.Tab href={metadataHref} data-test-crates-tab>Admin</NavTabs.Tab>
  </NavTabs.Root>
{/if}

<style>
  .layout {
    display: flex;
    align-items: center;
    gap: var(--space-s);
  }

  .identity {
    flex: 1;
    min-width: 0;
  }

  h1 {
    margin: 0;
    line-height: 1.1;
    overflow-wrap: anywhere;
    display: flex;
    gap: var(--space-3xs);
  }

  .display-name {
    margin-top: var(--space-3xs);
    color: var(--main-color-light);
    font-size: 0.9375em;
    overflow-wrap: anywhere;
  }

  .accounts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2xs);
    margin-top: var(--space-2xs);
  }

  :global(.user-page-avatar) {
    align-self: start;
    flex-shrink: 0;
    border-radius: 50%;
    object-fit: cover;
    background: white;
    padding: 3px;
    box-shadow: 1px 2px 2px 0 light-dark(hsla(51, 50%, 44%, 0.35), #232321);
  }
</style>

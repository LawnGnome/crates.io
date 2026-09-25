<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { SvelteDate } from 'svelte/reactivity';

  import { invalidate } from '$app/navigation';
  import { createClient } from '@crates-io/api-client';

  import { getNotifications } from '$lib/notifications.svelte';
  import Expiry from '../Expiry.svelte';
  import Panel from '../Panel.svelte';
  import PrivilegedAction from '../PrivilegedAction.svelte';

  let notifications = getNotifications();
  let client = createClient({ fetch });

  interface Props {
    user: components['schemas']['User'];
  }

  let id = $props.id();
  let { user }: Props = $props();

  let isUpdating = $state(false);
  let reason = $state(
    'You have violated the crates.io usage policies. Please contact help@crates.io to get your account restored.',
  );
  let expiry: SvelteDate | undefined = $state();
  let expiryInvalid = $state(false);

  async function onSubmit(event: SubmitEvent) {
    event.preventDefault();

    if (expiryInvalid || reason.length === 0) {
      return;
    }

    isUpdating = true;

    try {
      let result = await client.PUT('/api/v1/users/{user}/lock', {
        params: { path: { user: user.login } },
        body: {
          reason,
          until: expiry?.toISOString() ?? null,
        },
      });

      if (!result.response.ok) {
        throw new Error('Failed to lock the user');
      }

      await invalidate(url => {
        return url.pathname.startsWith('/api/v1/users/') && url.pathname.endsWith('/lock');
      });
    } catch {
      notifications.error('Unable to lock the user.');
    } finally {
      isUpdating = false;
    }
  }
</script>

<Panel class="p-m">
  <form onsubmit={onSubmit}>
    <div class="label">
      <label for="{id}-reason">Reason</label>
    </div>
    <div>
      <input
        type="text"
        id="{id}-reason"
        class="base-input input reason"
        required
        minlength="1"
        bind:value={reason}
        disabled={isUpdating}
        data-test-reason
      />
    </div>

    <div class="label">
      <label for="{id}-expiry">Expiry</label>
    </div>
    <div>
      <Expiry bind:date={expiry} id="{id}-expiry" noun="lock" />
    </div>

    <div>
      <PrivilegedAction>
        <button disabled={isUpdating} type="submit" class="button button--small button--red" data-test-lock>
          Lock {user.login}
        </button>
      </PrivilegedAction>
    </div>
  </form>
</Panel>

<style>
  form {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-2xs) var(--space-s);
    align-items: center;
  }

  form > div {
    grid-column: 2 / 3;
  }

  .label {
    grid-column: 1 / 2;
    font-weight: bold;
  }

  .reason {
    width: 100%;
  }
</style>

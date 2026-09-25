<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { invalidate } from '$app/navigation';
  import { createClient } from '@crates-io/api-client';

  import { getNotifications } from '$lib/notifications.svelte';
  import Panel from '../Panel.svelte';
  import PrivilegedAction from '../PrivilegedAction.svelte';

  let notifications = getNotifications();
  let client = createClient({ fetch });

  interface Props {
    user: components['schemas']['User'];
  }

  let { user }: Props = $props();

  let isUpdating = $state(false);

  async function onClick() {
    isUpdating = true;

    try {
      let result = await client.DELETE('/api/v1/users/{user}/lock', { params: { path: { user: user.login } } });

      if (!result.response.ok) {
        throw new Error('Failed to unlock the user');
      }

      await invalidate(url => {
        return url.pathname.startsWith('/api/v1/users/') && url.pathname.endsWith('/lock');
      });
    } catch {
      notifications.error('Unable to unlock the user.');
    } finally {
      isUpdating = false;
    }
  }
</script>

<Panel class="p-m">
  <PrivilegedAction>
    <button disabled={isUpdating} type="button" class="button button--small button--red" onclick={onClick}
      >Unlock {user.login}</button
    >
  </PrivilegedAction>
</Panel>

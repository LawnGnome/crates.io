<!-- SessionProvider.svelte -->
<script lang="ts" generics="Props extends Record<string, unknown>">
  import type { Component } from 'svelte';

  import { createClient } from '@crates-io/api-client';

  import { NotificationsState, setNotifications } from '$lib/notifications.svelte';
  import { SessionState, setSession } from '$lib/utils/session.svelte';

  let {
    session,
    notifications,
    component: Comp,
    props,
  }: {
    session?: SessionState;
    notifications?: NotificationsState;
    component: Component<Props>;
    props: Props;
  } = $props();

  // svelte-ignore state_referenced_locally
  setNotifications(notifications || new NotificationsState());

  // svelte-ignore state_referenced_locally
  setSession(session || new SessionState(createClient()));
</script>

<Comp {...props} />

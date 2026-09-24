<!-- SessionProvider.svelte -->
<script lang="ts" generics="Props extends Record<string, unknown>">
  import type { Component } from 'svelte';

  import { createClient } from '@crates-io/api-client';

  import { SessionState, setSession } from '$lib/utils/session.svelte';

  let {
    session,
    component: Comp,
    props,
  }: { session?: SessionState; component: Component<Props>; props: Props } = $props();

  // svelte-ignore state_referenced_locally
  setSession(session || new SessionState(createClient()));
</script>

<Comp {...props} />

<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { UserPageHeaderUserLock } from '../UserPageHeader.svelte';

  import { format, formatDistanceToNow } from 'date-fns';

  import Icon from '../Icon.svelte';
  import Panel from '../Panel.svelte';

  interface Props {
    lock?: UserPageHeaderUserLock | null;
    user: components['schemas']['User'];
  }

  let { lock, user }: Props = $props();
</script>

<Panel class="p-m">
  <dl class="metadata">
    <dt>ID</dt>
    <dd data-test-id>{user.id}</dd>
    <dt>Created at</dt>
    <dd data-test-created-at>
      {#if user.created_at}
        {format(user.created_at, 'PPP')}
        ({formatDistanceToNow(user.created_at)} ago)
      {:else}
        Unknown
      {/if}
    </dd>
    <dt>Lock status</dt>
    {#if lock}
      <dd class="locked" data-test-locked>
        <div class="blurb">
          <Icon class="i-mdi:lock" />
          Locked
        </div>
        <div>{lock.reason}</div>
        <div>
          {#if lock.until}
            Until {format(lock.until, 'PPP')}
            (in {formatDistanceToNow(lock.until)})
          {:else}
            Indefinite
          {/if}
        </div>
      </dd>
    {:else}
      <dd class="unlocked blurb" data-test-unlocked>
        <Icon class="i-mdi:lock-open" />
        Unlocked
      </dd>
    {/if}
  </dl>
</Panel>

<style>
  .metadata,
  .locked {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-2xs) var(--space-s);
  }

  dt {
    grid-column: 1 / 2;
    font-weight: bold;
  }

  dd {
    grid-column: 2 / 3;
  }

  .blurb {
    display: flex;
    gap: var(--space-2xs);
  }

  .locked {
    display: flex;
    flex-direction: column;
    gap: var(--space-2xs);
  }

  .locked .blurb {
    color: light-dark(var(--orange-700), var(--orange-300));
  }

  .unlocked {
    color: light-dark(var(--green800), hsl(115, 31%, 70%));
  }
</style>

<script lang="ts">
  import UserAdminLock from '$lib/components/user-admin/UserAdminLock.svelte';
  import UserAdminMetadata from '$lib/components/user-admin/UserAdminMetadata.svelte';
  import UserAdminUnlock from '$lib/components/user-admin/UserAdminUnlock.svelte';

  let { data } = $props();
</script>

<h2>Metadata</h2>

<UserAdminMetadata lock={data.lock} user={data.user} />

<h2>Actions</h2>

{#if data.lock}
  <UserAdminUnlock lock={data.lock} user={data.user} />
{:else}
  <UserAdminLock lock={data.lock} user={data.user} />
{/if}

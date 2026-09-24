<script lang="ts">
  import UserPageHeader from '$lib/components/UserPageHeader.svelte';

  let { children, data } = $props();
</script>

<UserPageHeader user={data.user} linkedAccounts={data.linkedAccounts ?? []} lock={data.lock} />

{@render children()}

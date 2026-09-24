<script lang="ts">
  import UserAdminMetadata from '$lib/components/user-admin/UserAdminMetadata.svelte';

  let { data } = $props();
</script>

<h2>Metadata</h2>

<UserAdminMetadata lock={data.lock} user={data.user} />

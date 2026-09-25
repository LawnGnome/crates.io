<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { DEFINITE_LOCK, INDEFINITE_LOCK, USER } from './test-data.svelte';
  import UserAdminMetadata from './UserAdminMetadata.svelte';

  const { Story } = defineMeta({
    title: 'user-admin/UserAdminMetadata',
    component: UserAdminMetadata,
    tags: ['autodocs'],
  });
</script>

<Story name="Combined" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <h1>Unlocked</h1>
  <UserAdminMetadata user={USER} />

  <h1>Locked indefinitely</h1>
  <UserAdminMetadata user={USER} lock={INDEFINITE_LOCK} />

  <h1>Locked until a time</h1>
  <UserAdminMetadata user={USER} lock={DEFINITE_LOCK} />
</Story>

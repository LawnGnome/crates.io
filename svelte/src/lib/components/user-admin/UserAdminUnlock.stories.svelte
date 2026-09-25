<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import SessionDecorator from '$lib/storybook/SessionDecorator.svelte';
  import { ADMIN_USER, USER } from './test-data.svelte';
  import UserAdminUnlock from './UserAdminUnlock.svelte';

  const { Story } = defineMeta({
    title: 'user-admin/UserAdminUnlock',
    component: UserAdminUnlock,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <SessionDecorator user={ADMIN_USER}>
    <UserAdminUnlock user={USER} />
  </SessionDecorator>
</Story>

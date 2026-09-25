<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import SessionDecorator from '$lib/storybook/SessionDecorator.svelte';
  import { ADMIN_USER, USER } from './test-data.svelte';
  import UserAdminLock from './UserAdminLock.svelte';

  const { Story } = defineMeta({
    title: 'user-admin/UserAdminLock',
    component: UserAdminLock,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <SessionDecorator user={ADMIN_USER}>
    <UserAdminLock user={USER} />
  </SessionDecorator>
</Story>

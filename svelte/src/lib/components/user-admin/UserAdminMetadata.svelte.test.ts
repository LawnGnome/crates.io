import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import { DEFINITE_LOCK, INDEFINITE_LOCK, USER } from './test-data.svelte';
import UserAdminMetadata from './UserAdminMetadata.svelte';

describe('UserAdminMetadata', () => {
  it('renders an unlocked user', async () => {
    await render(UserAdminMetadata, {
      user: USER,
    });

    let id = page.getByCSS('[data-test-id]');
    expect(id.element()).toMatchTextContent('1');

    let createdAt = page.getByCSS('[data-test-created-at]');
    expect(createdAt.element()).toMatchTextContent('7 days ago');

    let unlocked = page.getByCSS('[data-test-unlocked]');
    expect(unlocked.elements()).toHaveLength(1);

    let locked = page.getByCSS('[data-test-locked]');
    expect(locked.elements()).toHaveLength(0);
  });

  it('renders an indefinitely locked user', async () => {
    await render(UserAdminMetadata, {
      user: USER,
      lock: INDEFINITE_LOCK,
    });

    let id = page.getByCSS('[data-test-id]');
    expect(id.element()).toMatchTextContent('1');

    let createdAt = page.getByCSS('[data-test-created-at]');
    expect(createdAt.element()).toMatchTextContent('7 days ago');

    let unlocked = page.getByCSS('[data-test-unlocked]');
    expect(unlocked.elements()).toHaveLength(0);

    let locked = page.getByCSS('[data-test-locked]');
    expect(locked.element()).toMatchTextContent('Indefinite');
  });

  it('renders a user who is locked for a period of time', async () => {
    await render(UserAdminMetadata, {
      user: USER,
      lock: DEFINITE_LOCK,
    });

    let id = page.getByCSS('[data-test-id]');
    expect(id.element()).toMatchTextContent('1');

    let createdAt = page.getByCSS('[data-test-created-at]');
    expect(createdAt.element()).toMatchTextContent('7 days ago');

    let unlocked = page.getByCSS('[data-test-unlocked]');
    expect(unlocked.elements()).toHaveLength(0);

    let locked = page.getByCSS('[data-test-locked]');
    expect(locked.element()).toMatchTextContent('in 7 days');
  });
});

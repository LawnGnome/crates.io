import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import { NotFoundError } from '@ember-data/adapter/error';

export default class AdminUserCratesRoute extends Route {
  @service router;
  @service store;

  queryParams = { page: { refreshModel: true } };

  async model(params, transition) {
    const { user } = this.modelFor('admin.user');

    try {
      params.user_id = user.get('id');
      params.include_yanked = 'yes';
      const crates = await this.store.query('crate', params);

      return { crates, user };
    } catch (error) {
      if (error instanceof NotFoundError) {
        let title = `${user_id}: User not found`;
        this.router.replaceWith('catch-all', { transition, error, title });
      } else {
        let title = `${user_id}: Failed to load user data`;
        this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
      }
    }
  }
}

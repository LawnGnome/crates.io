import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import ajax, { HttpError } from '../../utils/ajax';

export default class AdminUserRoute extends Route {
  @service router;
  @service store;

  async model(params, transition) {
    const { user_id } = params;

    try {
      // We need to get the admin view of the user.
      const response = await ajax(`/api/v1/users/${user_id}/admin`);

      return { user: this.store.push(this.store.normalize('user', response)) };
    } catch (error) {
      if (error instanceof HttpError && error.response?.status === 404) {
        this.router.replaceWith('catch-all', {
          transition,
          error,
          title: `${user_id}: User not found`,
        });
      } else {
        this.router.replaceWith('catch-all', {
          transition,
          error,
          title: `${user_id}: Failed to load user data`,
          tryAgain: true,
        });
      }
    }
  }
}

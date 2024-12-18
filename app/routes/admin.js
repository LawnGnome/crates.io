import AuthenticatedRoute from './-authenticated-route';
import { inject as service } from '@ember/service';

export default class AdminRoute extends AuthenticatedRoute {
  async beforeModel(transition) {
    await super.beforeModel(transition);

    if (!this.session.isAdmin) {
      this.session.savedTransition = transition;

      this.router.replaceWith('catch-all', {
        transition,
        loginNeeded: true,
        title: 'This page requires authentication',
      });
    }
  }
}

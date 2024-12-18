import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

export default class AdminIndexController extends Controller {
  @service router;
  @tracked username;

  @action searchUser() {
    this.router.transitionTo('admin.user', { user_id: this.username });
  }
}

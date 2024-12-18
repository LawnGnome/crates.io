import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

import { addDays, isFuture } from 'date-fns';

import ajax from '../../../utils/ajax';

export default class AdminUserProfileController extends Controller {
  @service notifications;
  @service sentry;
  @service store;

  @tracked days;
  @tracked reason;

  constructor() {
    super(...arguments);
    this.reset();
  }

  reset() {
    this.days = '7';
    this.reason = 'Please contact help@crates.io.';
  }

  get isCurrentlyLocked() {
    if (!this.lockReason) {
      return false;
    }

    const until = this.lockedUntil;
    if (!until) {
      return true;
    }

    return isFuture(until);
  }

  get wasPreviouslyLocked() {
    return this.lockReason && !this.isCurrentlyLocked;
  }

  get lockReason() {
    return this.model.user.lock?.reason;
  }

  get lockedUntil() {
    return this.model.user.lock?.until;
  }

  @action updateDays(event) {
    this.days = event.target.value;
  }

  asPayload() {
    let { reason, days } = this;

    let until = null;
    days = new Number(days);
    if (!isNaN(days)) {
      until = addDays(new Date(), days);
    }

    return { reason, until };
  }

  lockTask = task(async () => {
    try {
      const user = await ajax(`/api/v1/users/${this.model.user.login}/lock`, {
        method: 'PUT',
        body: JSON.stringify(this.asPayload()),
        headers: {
          'Content-Type': 'application/json',
        },
      });

      this.notifications.success('User account locked');
      this.reset();
      this.store.push(this.store.normalize('user', user));
    } catch (e) {
      this.notifications.error('An error occurred while locking the account.');
      this.sentry.captureException(e);
      console.error(e);
    }
  });

  unlockTask = task(async () => {
    try {
      const user = await ajax(`/api/v1/users/${this.model.user.login}/lock`, { method: 'DELETE' });

      this.notifications.success('User account unlocked');
      this.reset();
      this.store.push(this.store.normalize('user', user));
    } catch (e) {
      this.notifications.error('An error occurred while unlocking the account.');
      this.sentry.captureException(e);
      console.error(e);
    }
  });
}

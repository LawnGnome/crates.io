import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { pagination } from '../../../utils/pagination';

export default class AdminUserCratesController extends Controller {
  queryParams = ['page', 'per_page'];
  @tracked page = '1';
  @tracked per_page = 10;

  @pagination() pagination;

  get totalItems() {
    return this.model.crates.meta.total ?? 0;
  }
}

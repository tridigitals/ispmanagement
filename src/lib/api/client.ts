import { announcements } from './announcements';
import { audit } from './audit';
import { auth } from './auth';
import { backup } from './backup';
import { customers } from './customers';
import { emailOutbox } from './emailOutbox';
import { install } from './install';
import { ispPackages } from './ispPackages';
import { mikrotik } from './mikrotik';
import { networkMapping } from './networkMapping';
import { notifications } from './notifications';
import { payment } from './payment';
import { plans } from './plans';
import { pppoe } from './pppoe';
import { publicApi } from './public';
import { roles } from './roles';
import { settings } from './settings';
import { storage } from './storage';
import { support } from './support';
import { superadmin } from './superadmin';
import { tenant } from './tenant';
import { team } from './team';
import { users } from './users';
import { workOrders } from './workOrders';
export { announcements } from './announcements';
export { audit } from './audit';
export { auth } from './auth';
export { backup } from './backup';
export { customers } from './customers';
export { emailOutbox } from './emailOutbox';
export { install } from './install';
export { ispPackages } from './ispPackages';
export { mikrotik } from './mikrotik';
export { networkMapping } from './networkMapping';
export { notifications } from './notifications';
export { payment } from './payment';
export { plans } from './plans';
export { pppoe } from './pppoe';
export { publicApi } from './public';
export { roles } from './roles';
export { settings } from './settings';
export { storage } from './storage';
export { support } from './support';
export { superadmin } from './superadmin';
export { tenant } from './tenant';
export { team } from './team';
export { users } from './users';
export { workOrders } from './workOrders';
export type * from './types';
// Combined API object
export const api = {
  auth,
  users,
  roles,
  team,
  customers,
  workOrders,
  pppoe,
  ispPackages,
  networkMapping,
  superadmin,
  audit,
  mikrotik,
  support,
  announcements,
  settings,
  install,
  plans,
  storage,
  payment,
  tenant,
  notifications,
  emailOutbox,
  backup,
};

export default api;

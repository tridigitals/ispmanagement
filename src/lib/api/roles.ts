import { getTokenOrThrow, safeInvoke } from './core';
import type { Permission, Role } from './types';

export const roles = {
  list: (): Promise<Role[]> => safeInvoke('get_roles', { token: getTokenOrThrow() }),

  getPermissions: (): Promise<Permission[]> =>
    safeInvoke('get_permissions', { token: getTokenOrThrow() }),

  get: (id: string): Promise<Role | null> =>
    safeInvoke('get_role', { token: getTokenOrThrow(), id, roleId: id }),

  create: (
    name: string,
    description: string | undefined,
    level: number,
    permissions: string[],
  ): Promise<Role> =>
    safeInvoke('create_new_role', {
      token: getTokenOrThrow(),
      name,
      description,
      level,
      permissions,
    }),

  update: (
    id: string,
    name?: string,
    description?: string,
    level?: number,
    permissions?: string[],
  ): Promise<Role> =>
    safeInvoke('update_existing_role', {
      token: getTokenOrThrow(),
      id,
      roleId: id,
      name,
      description,
      level,
      permissions,
    }),

  delete: (id: string): Promise<boolean> =>
    safeInvoke('delete_existing_role', { token: getTokenOrThrow(), id, roleId: id }),
};

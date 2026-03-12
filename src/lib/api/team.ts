import { getTokenOrThrow, safeInvoke } from './core';
import type { TeamMember } from './types';

export const team = {
  list: (): Promise<TeamMember[]> => safeInvoke('list_team_members', { token: getTokenOrThrow() }),

  add: (email: string, name: string, roleId: string, password?: string): Promise<TeamMember> =>
    safeInvoke('add_team_member', { token: getTokenOrThrow(), email, name, roleId, password }),

  updateRole: (memberId: string, roleId: string): Promise<void> =>
    safeInvoke('update_team_member_role', {
      token: getTokenOrThrow(),
      id: memberId,
      memberId,
      roleId,
    }),

  remove: (memberId: string): Promise<void> =>
    safeInvoke('remove_team_member', { token: getTokenOrThrow(), id: memberId, memberId }),
};

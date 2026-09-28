import { getTokenOrThrow, safeInvoke } from './core';
import type { TeamMember } from './types';

export const team = {
  list: (): Promise<TeamMember[]> => safeInvoke('list_team_members', { token: getTokenOrThrow() }),

  /**
   * `isActive` dan `emailVerified` menentukan status akun sejak dibuat.
   * Keduanya opsional; bila kosong, akun dibuat aktif dan emailnya dianggap
   * sudah terverifikasi (admin menambahkannya sendiri, dan belum ada email
   * verifikasi yang dikirim).
   */
  add: (
    email: string,
    name: string,
    roleId: string,
    password?: string,
    opts?: { isActive?: boolean; emailVerified?: boolean },
  ): Promise<TeamMember> =>
    safeInvoke('add_team_member', {
      token: getTokenOrThrow(),
      email,
      name,
      roleId,
      password,
      is_active: opts?.isActive,
      email_verified: opts?.emailVerified,
    }),

  /**
   * Perbarui anggota tim. Ketiga field opsional dan independen — mengubah
   * status akun saja tidak menuntut ikut mengganti role.
   */
  update: (
    memberId: string,
    data: { roleId?: string; isActive?: boolean; emailVerified?: boolean },
  ): Promise<void> =>
    safeInvoke('update_team_member_role', {
      token: getTokenOrThrow(),
      id: memberId,
      memberId,
      roleId: data.roleId,
      is_active: data.isActive,
      email_verified: data.emailVerified,
    }),

  /** Nama lama dipertahankan supaya pemanggil lama tidak putus. */
  updateRole: (memberId: string, roleId: string): Promise<void> =>
    safeInvoke('update_team_member_role', {
      token: getTokenOrThrow(),
      id: memberId,
      memberId,
      roleId,
    }),

  remove: (memberId: string): Promise<void> =>
    safeInvoke('remove_team_member', { token: getTokenOrThrow(), id: memberId, memberId }),

  listDeleted: (): Promise<TeamMember[]> =>
    safeInvoke('list_deleted_team_members', { token: getTokenOrThrow() }),

  restore: (memberId: string): Promise<void> =>
    safeInvoke('restore_team_member', { token: getTokenOrThrow(), memberId }),

  hardDelete: (memberId: string): Promise<void> =>
    safeInvoke('hard_delete_team_member', { token: getTokenOrThrow(), memberId }),
};

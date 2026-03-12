import { getTokenOrThrow, safeInvoke } from './core';
import type {
  Announcement,
  CreateAnnouncementDto,
  PaginatedResponse,
  UpdateAnnouncementDto,
} from './types';

export const announcements = {
  listActive: (): Promise<Announcement[]> =>
    safeInvoke('list_active_announcements', { token: getTokenOrThrow() }),

  listRecent: (params?: {
    page?: number;
    per_page?: number;
    search?: string;
    severity?: string;
    mode?: string;
  }): Promise<PaginatedResponse<Announcement>> =>
    safeInvoke('list_recent_announcements', { token: getTokenOrThrow(), ...(params || {}) }),

  get: (id: string): Promise<Announcement> =>
    safeInvoke('get_announcement', { token: getTokenOrThrow(), id }),

  dismiss: (id: string): Promise<void> =>
    safeInvoke('dismiss_announcement', { token: getTokenOrThrow(), id }),

  listAdmin: (params?: {
    scope?: 'tenant' | 'global' | 'all';
    page?: number;
    per_page?: number;
    search?: string;
    severity?: string;
    mode?: string;
    status?: string;
  }): Promise<PaginatedResponse<Announcement>> =>
    safeInvoke('list_announcements_admin', { token: getTokenOrThrow(), ...(params || {}) }),

  createAdmin: (dto: CreateAnnouncementDto): Promise<Announcement> =>
    safeInvoke('create_announcement_admin', { token: getTokenOrThrow(), ...dto }),

  updateAdmin: (id: string, dto: UpdateAnnouncementDto): Promise<Announcement> =>
    safeInvoke('update_announcement_admin', { token: getTokenOrThrow(), id, ...dto }),

  deleteAdmin: (id: string): Promise<void> =>
    safeInvoke('delete_announcement_admin', { token: getTokenOrThrow(), id }),
};

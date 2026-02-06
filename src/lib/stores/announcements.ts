import { writable, get } from 'svelte/store';
import { api } from '$lib/api/client';

export type Announcement = {
  id: string;
  tenant_id: string | null;
  created_by: string | null;
  title: string;
  body: string;
  severity: 'info' | 'success' | 'warning' | 'error' | string;
  audience: 'all' | 'admins' | string;
  starts_at: string;
  ends_at: string | null;
  notified_at: string | null;
  created_at: string;
  updated_at: string;
};

export const activeAnnouncements = writable<Announcement[]>([]);
export const announcementsLoading = writable(false);

export async function loadActiveAnnouncements() {
  announcementsLoading.set(true);
  try {
    const rows = await api.announcements.listActive();
    activeAnnouncements.set(rows as any);
  } catch (e) {
    // non-blocking
    console.warn('Failed to load announcements:', e);
  } finally {
    announcementsLoading.set(false);
  }
}

export async function dismissAnnouncement(id: string) {
  // Optimistic remove
  activeAnnouncements.update((items) => items.filter((a) => a.id !== id));
  try {
    await api.announcements.dismiss(id);
  } catch (e) {
    console.warn('Failed to dismiss announcement:', e);
    // Reload to sync
    await loadActiveAnnouncements();
  }
}

export function hasActiveAnnouncements() {
  return get(activeAnnouncements).length > 0;
}


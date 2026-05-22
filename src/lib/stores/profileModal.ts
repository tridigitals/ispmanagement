import { writable } from 'svelte/store';

export const PROFILE_MODAL_TABS = [
  'general',
  'security',
  'preferences',
  'notifications',
  'addresses',
] as const;

export type ProfileTabId = (typeof PROFILE_MODAL_TABS)[number];
export type ProfileModalReason = 'manual' | '2fa_required' | null;

type ProfileModalState = {
  open: boolean;
  tab: ProfileTabId;
  locked: boolean;
  reason: ProfileModalReason;
};

const initialState: ProfileModalState = {
  open: false,
  tab: 'general',
  locked: false,
  reason: null,
};

export const profileModal = writable<ProfileModalState>({ ...initialState });

export function openProfileModal(options?: {
  tab?: ProfileTabId;
  locked?: boolean;
  reason?: Exclude<ProfileModalReason, null>;
}) {
  profileModal.update((state) => ({
    open: true,
    tab: options?.tab || state.tab || 'general',
    locked: options?.locked ?? false,
    reason: options?.reason ?? 'manual',
  }));
}

export function closeProfileModal() {
  profileModal.update((state) => {
    if (state.locked) return state;
    return { ...initialState };
  });
}

export function setProfileModalTab(tab: ProfileTabId) {
  profileModal.update((state) => ({ ...state, tab }));
}

export function setProfileModalLock(locked: boolean, reason: ProfileModalReason = null) {
  profileModal.update((state) => ({
    ...state,
    locked,
    reason: locked ? reason || state.reason || '2fa_required' : null,
  }));
}

export function resetProfileModal() {
  profileModal.set({ ...initialState });
}

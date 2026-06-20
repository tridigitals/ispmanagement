import { writable } from 'svelte/store';

type NotificationModalState = {
  open: boolean;
};

const initialState: NotificationModalState = {
  open: false,
};

export const notificationModal = writable<NotificationModalState>({ ...initialState });

export function openNotificationModal() {
  notificationModal.set({ open: true });
}

export function closeNotificationModal() {
  notificationModal.set({ ...initialState });
}

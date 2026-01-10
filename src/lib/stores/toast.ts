import { writable } from 'svelte/store';

function generateId() {
    return crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);
}

export type ToastType = 'success' | 'error' | 'info' | 'warning';

export interface Toast {
    id: string;
    message: string;
    type: ToastType;
    duration?: number;
}

function createToastStore() {
    const { subscribe, update } = writable<Toast[]>([]);

    return {
        subscribe,
        add: (message: string, type: ToastType = 'info', duration: number = 3000) => {
            const id = generateId();
            const toast: Toast = { id, message, type, duration };

            update((toasts) => [...toasts, toast]);

            if (duration > 0) {
                setTimeout(() => {
                    update((toasts) => toasts.filter((t) => t.id !== id));
                }, duration);
            }
        },
        remove: (id: string) => {
            update((toasts) => toasts.filter((t) => t.id !== id));
        },
        success: (message: string, duration = 3000) => {
            toast.add(message, 'success', duration);
        },
        error: (message: string, duration = 5000) => {
            toast.add(message, 'error', duration);
        },
        info: (message: string, duration = 3000) => {
            toast.add(message, 'info', duration);
        },
        warning: (message: string, duration = 4000) => {
            toast.add(message, 'warning', duration);
        }
    };
}

export const toast = createToastStore();

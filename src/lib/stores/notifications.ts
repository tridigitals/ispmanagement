import { writable, get } from 'svelte/store';
import { notifications as api, type Notification, type NotificationPreference } from '$lib/api/client';
import { toast } from '@zerodevx/svelte-toast';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { isTauri } from '@tauri-apps/api/core';

// Helper to convert VAPID key
function urlBase64ToUint8Array(base64String: string) {
    const padding = '='.repeat((4 - base64String.length % 4) % 4);
    const base64 = (base64String + padding)
        .replace(/\-/g, '+')
        .replace(/_/g, '/');

    const rawData = window.atob(base64);
    const outputArray = new Uint8Array(rawData.length);

    for (let i = 0; i < rawData.length; ++i) {
        outputArray[i] = rawData.charCodeAt(i);
    }
    return outputArray;
}

// Stores
export const notifications = writable<Notification[]>([]);
export const unreadCount = writable<number>(0);
export const loading = writable<boolean>(false);
export const preferences = writable<NotificationPreference[]>([]);
export const pushEnabled = writable<boolean>(false); // Tracks active subscription status

// State for pagination
export const pagination = writable({
    page: 1,
    perPage: 20,
    total: 0,
    hasMore: false,
});

/**
 * Fetch notifications
 */
export async function loadNotifications(page: number = 1, append: boolean = false) {
    loading.set(true);
    try {
        const perPage = get(pagination).perPage;
        const res = await api.list(page, perPage);

        if (append) {
            notifications.update(curr => [...curr, ...res.data]);
        } else {
            notifications.set(res.data);
        }

        pagination.set({
            page: res.page,
            perPage: res.per_page,
            total: res.total,
            hasMore: res.data.length === perPage, // Simple check, could be better
        });

        // Also refresh count
        refreshUnreadCount();

    } catch (e) {
        console.error('Failed to load notifications:', e);
        // toast.push('Failed to load notifications');
    } finally {
        loading.set(false);
    }
}

/**
 * Refresh unread count
 */
export async function refreshUnreadCount() {
    try {
        const res = await api.getUnreadCount();
        unreadCount.set(res.count);
    } catch (e) {
        console.error('Failed to get unread count:', e);
    }
}

/**
 * Mark a notification as read
 */
export async function markAsRead(id: string) {
    // Optimistic update
    notifications.update(items => items.map(n =>
        n.id === id ? { ...n, is_read: true } : n
    ));
    // Decrement count proactively
    unreadCount.update(c => Math.max(0, c - 1));

    try {
        await api.markAsRead(id);
    } catch (e) {
        // Revert on error?
        console.error('Failed to mark as read:', e);
        refreshUnreadCount(); // Sync back
    }
}

/**
 * Mark all as read
 */
export async function markAllAsRead() {
    // Optimistic
    notifications.update(items => items.map(n => ({ ...n, is_read: true })));
    unreadCount.set(0);

    try {
        await api.markAllAsRead();
    } catch (e) {
        console.error('Failed to mark all as read:', e);
        loadNotifications(1); // Reload to sync
        refreshUnreadCount();
    }
}

/**
 * Delete a notification
 */
export async function deleteNotification(id: string) {
    // Optimistic remove
    notifications.update(items => items.filter(n => n.id !== id));

    try {
        await api.delete(id);
    } catch (e) {
        console.error('Failed to delete notification:', e);
        loadNotifications(1); // Reload
    }
}

/**
 * Load user preferences
 */
export async function loadPreferences() {
    try {
        const res = await api.getPreferences();
        preferences.set(res);
    } catch (e) {
        console.error('Failed to load preferences:', e);
    }
}

/**
 * Update preference
 */
export async function updatePreference(channel: string, category: string, enabled: boolean) {
    // Optimistic
    preferences.update(prefs => {
        const idx = prefs.findIndex(p => p.channel === channel && p.category === category);
        if (idx !== -1) {
            const newPrefs = [...prefs];
            newPrefs[idx] = { ...newPrefs[idx], enabled };
            return newPrefs;
        }
        return prefs;
    });

    try {
        await api.updatePreference(channel, category, enabled);
    } catch (e) {
        console.error('Failed to update preference:', e);
        loadPreferences(); // Revert
    }
}

/**
 * Check if push is enabled
 */
export async function checkSubscription() {
    if (!('serviceWorker' in navigator)) {
        pushEnabled.set(false);
        return;
    }

    try {
        const registration = await navigator.serviceWorker.ready;
        const subscription = await registration.pushManager.getSubscription();
        pushEnabled.set(!!subscription);
    } catch (e) {
        console.error('Failed to check push subscription:', e);
        pushEnabled.set(false);
    }
}

/**
 * Subscribe to Push Notifications
 */
export async function subscribePush() {
    if (!('serviceWorker' in navigator) || !('PushManager' in window)) {
        toast.push({ msg: 'Push notifications not supported', theme: { '--toastBackground': 'var(--color-danger)' } });
        return;
    }

    // 1. Check if blocked
    if (Notification.permission === 'denied') {
        toast.push({
            msg: 'Notifications are blocked! Please click the lock icon in your URL bar and "Reset Permission".',
            theme: { '--toastBackground': 'var(--color-danger)' }
        });
        return;
    }

    try {
        // 2. Request permission explicitly
        const permission = await Notification.requestPermission();
        if (permission !== 'granted') {
            toast.push({ msg: 'Permission denied. You need to allow notifications.', theme: { '--toastBackground': 'var(--color-warning)' } });
            return;
        }

        const registration = await navigator.serviceWorker.ready;
        const vapidPublicKey = import.meta.env.VITE_VAPID_PUBLIC_KEY;

        if (!vapidPublicKey) {
            console.error('VAPID public key not found');
            toast.push({ msg: 'Configuration error: Missing VAPID key', theme: { '--toastBackground': 'var(--color-danger)' } });
            return;
        }

        // 3. Subscribe
        const subscription = await registration.pushManager.subscribe({
            userVisibleOnly: true,
            applicationServerKey: urlBase64ToUint8Array(vapidPublicKey)
        });

        // Send to backend
        const p256dh = subscription.getKey('p256dh');
        const auth = subscription.getKey('auth');

        if (p256dh && auth) {
            const toBase64Url = (arr: ArrayBuffer) => {
                return btoa(String.fromCharCode.apply(null, Array.from(new Uint8Array(arr))))
                    .replace(/\+/g, '-')
                    .replace(/\//g, '_')
                    .replace(/=+$/, '');
            };

            await api.subscribePush(
                subscription.endpoint,
                toBase64Url(p256dh),
                toBase64Url(auth)
            );
            pushEnabled.set(true);
            toast.push({ msg: 'Push notifications enabled successfully!' });
        } else {
            console.warn('Push subscription missing keys');
        }

    } catch (e) {
        console.error('Failed to subscribe to push:', e);
        toast.push({ msg: `Error: ${e instanceof Error ? e.message : 'Unknown error'}`, theme: { '--toastBackground': 'var(--color-danger)' } });
    }
}

/**
 * Unsubscribe from Push Notifications
 */
export async function unsubscribePush() {
    if (!('serviceWorker' in navigator)) return;

    try {
        const registration = await navigator.serviceWorker.ready;
        const subscription = await registration.pushManager.getSubscription();

        if (subscription) {
            await subscription.unsubscribe();
            await api.unsubscribePush(subscription.endpoint);
            pushEnabled.set(false);
            toast.push({ msg: 'Push notifications disabled' });
        }
    } catch (e) {
        console.error('Failed to unsubscribe push:', e);
    }
}

/**
 * Send Test Notification
 */
export async function sendTestNotification() {
    try {
        await api.sendTest();
        toast.push({ msg: 'Test notification sent!' });
    } catch (e) {
        console.error('Failed to send test notification:', e);
        toast.push({ msg: 'Failed to send test notification', theme: { '--toastBackground': 'var(--color-danger)' } });
    }
}

// --- WebSocket Event Handlers ---

export function handleNotificationReceived(notification: Notification) {
    // Add to top of list
    notifications.update(items => [notification, ...items]);
    // Increment unread count
    unreadCount.update(c => c + 1);

    // Show toast for in-app feedback
    toast.push({
        msg: notification.title,
        // TODO: Custom component for toast with action
    });

    // If Desktop, also trigger system notification
    // If Desktop, also trigger system notification
    if (isTauri()) {
        try {
            sendNotification({
                title: notification.title,
                body: notification.message || 'New notification received',
            });
        } catch (e) {
            console.error('Failed to send system notification:', e);
        }
    } else if (Notification.permission === 'granted') {
        // Validation for Browser: Trigger standard Web Notification
        // This ensures users get a system popup even if they don't have Background Push enabled,
        // as long as the tab is open. Using 'tag' prevents duplicates if Push is also received.
        try {
            const n = new Notification(notification.title, {
                body: notification.message || 'New notification received',
                tag: notification.id, // Match SW tag
                icon: '/icon-192x192.png'
            });
            n.onclick = () => {
                n.close();
                window.focus();
                // Optional: navigate if action_url exists
            };
        } catch (e) {
            console.error('Failed to trigger Web Notification:', e);
        }
    }
}

export function handleUnreadCountUpdated(count: number) {
    unreadCount.set(count);
}

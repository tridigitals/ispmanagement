import type { Component } from 'svelte';
import type { NotificationPreference } from '$lib/api/client';
import type { Readable } from 'svelte/store';

type DeferredComponent = Component<any>;
type AsyncModuleLoader<T> = () => Promise<T>;

type NotificationStoreModule = typeof import('$lib/stores/notifications');

export type ProfileNotificationsRuntime = {
  NotificationsTabComponent: DeferredComponent;
  preferencesStore: Readable<NotificationPreference[]>;
  pushEnabledStore: Readable<boolean>;
  loadPreferences: NotificationStoreModule['loadPreferences'];
  updatePreference: NotificationStoreModule['updatePreference'];
  subscribePush: NotificationStoreModule['subscribePush'];
  unsubscribePush: NotificationStoreModule['unsubscribePush'];
  sendTestNotification: NotificationStoreModule['sendTestNotification'];
  checkSubscription: NotificationStoreModule['checkSubscription'];
};

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export const loadProfileGeneralTab = createCachedLoader(() =>
  import('$lib/components/profile/ProfileGeneralTab.svelte'),
);

export const loadProfileSecurityTab = createCachedLoader(() =>
  import('$lib/components/profile/ProfileSecurityTab.svelte'),
);

export const loadProfilePreferencesTab = createCachedLoader(() =>
  import('$lib/components/profile/ProfilePreferencesTab.svelte'),
);

export const loadProfileAddressesTab = createCachedLoader(() =>
  import('$lib/components/profile/ProfileAddressesTab.svelte'),
);

export const loadProfileNotificationsRuntime = createCachedLoader(async () => {
  const [{ default: NotificationsTabComponent }, notificationsRuntime] = await Promise.all([
    import('$lib/components/profile/ProfileNotificationsTab.svelte'),
    import('$lib/stores/notifications'),
  ]);

  return {
    NotificationsTabComponent,
    preferencesStore: notificationsRuntime.preferences,
    pushEnabledStore: notificationsRuntime.pushEnabled,
    loadPreferences: notificationsRuntime.loadPreferences,
    updatePreference: notificationsRuntime.updatePreference,
    subscribePush: notificationsRuntime.subscribePush,
    unsubscribePush: notificationsRuntime.unsubscribePush,
    sendTestNotification: notificationsRuntime.sendTestNotification,
    checkSubscription: notificationsRuntime.checkSubscription,
  };
});

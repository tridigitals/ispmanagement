import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  general: { name: 'profile-general-tab' },
  security: { name: 'profile-security-tab' },
  preferences: { name: 'profile-preferences-tab' },
  notifications: { name: 'profile-notifications-tab' },
  addresses: { name: 'profile-addresses-tab' },
  notificationStores: {
    preferences: { subscribe: vi.fn() },
    pushEnabled: { subscribe: vi.fn() },
    loadPreferences: vi.fn(),
    updatePreference: vi.fn(),
    subscribePush: vi.fn(),
    unsubscribePush: vi.fn(),
    sendTestNotification: vi.fn(),
    checkSubscription: vi.fn(),
  },
}));

vi.mock('$lib/components/profile/ProfileGeneralTab.svelte', () => ({
  default: sentinels.general,
}));

vi.mock('$lib/components/profile/ProfileSecurityTab.svelte', () => ({
  default: sentinels.security,
}));

vi.mock('$lib/components/profile/ProfilePreferencesTab.svelte', () => ({
  default: sentinels.preferences,
}));

vi.mock('$lib/components/profile/ProfileNotificationsTab.svelte', () => ({
  default: sentinels.notifications,
}));

vi.mock('$lib/components/profile/ProfileAddressesTab.svelte', () => ({
  default: sentinels.addresses,
}));

vi.mock('$lib/stores/notifications', () => sentinels.notificationStores);

import {
  loadProfileAddressesTab,
  loadProfileGeneralTab,
  loadProfileNotificationsRuntime,
  loadProfilePreferencesTab,
  loadProfileSecurityTab,
} from './profilePageModules';

describe('profile page modules', () => {
  it('loads and caches each profile tab component on demand', async () => {
    const firstGeneral = await loadProfileGeneralTab();
    const secondGeneral = await loadProfileGeneralTab();
    const security = await loadProfileSecurityTab();
    const preferences = await loadProfilePreferencesTab();
    const addresses = await loadProfileAddressesTab();

    expect(firstGeneral).toEqual({ default: sentinels.general });
    expect(secondGeneral).toBe(firstGeneral);
    expect(security).toEqual({ default: sentinels.security });
    expect(preferences).toEqual({ default: sentinels.preferences });
    expect(addresses).toEqual({ default: sentinels.addresses });
  });

  it('loads and caches the notifications tab runtime lazily', async () => {
    const first = await loadProfileNotificationsRuntime();
    const second = await loadProfileNotificationsRuntime();

    expect(first.NotificationsTabComponent).toBe(sentinels.notifications);
    expect(first.preferencesStore).toBe(sentinels.notificationStores.preferences);
    expect(first.pushEnabledStore).toBe(sentinels.notificationStores.pushEnabled);
    expect(first.loadPreferences).toBe(sentinels.notificationStores.loadPreferences);
    expect(first.updatePreference).toBe(sentinels.notificationStores.updatePreference);
    expect(first.subscribePush).toBe(sentinels.notificationStores.subscribePush);
    expect(first.unsubscribePush).toBe(sentinels.notificationStores.unsubscribePush);
    expect(first.sendTestNotification).toBe(sentinels.notificationStores.sendTestNotification);
    expect(first.checkSubscription).toBe(sentinels.notificationStores.checkSubscription);
    expect(second).toBe(first);
  });
});

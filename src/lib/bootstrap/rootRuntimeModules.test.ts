import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  themeModule: {
    theme: {
      init: vi.fn(),
    },
  },
  installModule: {
    install: {
      checkIsInstalled: vi.fn(),
    },
  },
  websocketModule: {
    connectWebSocket: vi.fn(),
    disconnectWebSocket: vi.fn(),
  },
  notificationsModule: {
    loadNotifications: vi.fn(),
    refreshUnreadCount: vi.fn(),
    resetNotificationsState: vi.fn(),
  },
}));

vi.mock('$lib/stores/theme', () => sentinels.themeModule);
vi.mock('$lib/api/install', () => sentinels.installModule);
vi.mock('$lib/stores/websocket', () => sentinels.websocketModule);
vi.mock('$lib/stores/notifications', () => sentinels.notificationsModule);

import {
  loadInstallModule,
  loadRealtimeRuntime,
  loadThemeModule,
} from './rootRuntimeModules';

describe('root runtime modules', () => {
  it('loads and caches the deferred theme module', async () => {
    const first = await loadThemeModule();
    const second = await loadThemeModule();

    expect(first.theme.init).toBe(sentinels.themeModule.theme.init);
    expect(second).toBe(first);
  });

  it('loads and caches the deferred install module', async () => {
    const first = await loadInstallModule();
    const second = await loadInstallModule();

    expect(first.install.checkIsInstalled).toBe(sentinels.installModule.install.checkIsInstalled);
    expect(second).toBe(first);
  });

  it('loads and caches the combined realtime runtime modules', async () => {
    const first = await loadRealtimeRuntime();
    const second = await loadRealtimeRuntime();

    expect(first.connectWebSocket).toBe(sentinels.websocketModule.connectWebSocket);
    expect(first.disconnectWebSocket).toBe(sentinels.websocketModule.disconnectWebSocket);
    expect(first.loadNotifications).toBe(sentinels.notificationsModule.loadNotifications);
    expect(first.refreshUnreadCount).toBe(sentinels.notificationsModule.refreshUnreadCount);
    expect(first.resetNotificationsState).toBe(sentinels.notificationsModule.resetNotificationsState);
    expect(second).toBe(first);
  });
});

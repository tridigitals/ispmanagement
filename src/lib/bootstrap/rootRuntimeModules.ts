type AsyncModuleLoader<T> = () => Promise<T>;

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export const loadThemeModule = createCachedLoader(() => import('$lib/stores/theme'));

export const loadInstallModule = createCachedLoader(() => import('$lib/api/install'));

export const loadRealtimeRuntime = createCachedLoader(async () => {
  const [websocketModule, notificationsModule] = await Promise.all([
    import('$lib/stores/websocket'),
    import('$lib/stores/notifications'),
  ]);

  return {
    connectWebSocket: websocketModule.connectWebSocket,
    disconnectWebSocket: websocketModule.disconnectWebSocket,
    loadNotifications: notificationsModule.loadNotifications,
    refreshUnreadCount: notificationsModule.refreshUnreadCount,
    resetNotificationsState: notificationsModule.resetNotificationsState,
  };
});

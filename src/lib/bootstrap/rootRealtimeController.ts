type RealtimeRuntime = {
  connectWebSocket: () => void;
  disconnectWebSocket: () => void;
  loadNotifications: (page?: number) => Promise<void>;
  refreshUnreadCount: () => void;
};

type LoadRealtimeRuntime = () => Promise<RealtimeRuntime>;

export function createRootRealtimeController(loadRuntime: LoadRealtimeRuntime) {
  let connected = false;
  let connectInFlight: Promise<void> | null = null;
  let disconnectInFlight: Promise<void> | null = null;
  let runtimePromise: Promise<RealtimeRuntime> | null = null;

  function getRuntime() {
    if (!runtimePromise) {
      runtimePromise = loadRuntime();
    }
    return runtimePromise;
  }

  return {
    async connect() {
      if (connected) return;
      if (connectInFlight) return connectInFlight;

      connectInFlight = (async () => {
        const runtime = await getRuntime();
        runtime.connectWebSocket();
        await runtime.loadNotifications(1);
        runtime.refreshUnreadCount();
        connected = true;
      })().finally(() => {
        connectInFlight = null;
      });

      return connectInFlight;
    },

    async disconnect() {
      if (!connected) return;
      if (disconnectInFlight) return disconnectInFlight;

      disconnectInFlight = (async () => {
        const runtime = await getRuntime();
        runtime.disconnectWebSocket();
        connected = false;
      })().finally(() => {
        disconnectInFlight = null;
      });

      return disconnectInFlight;
    },
  };
}

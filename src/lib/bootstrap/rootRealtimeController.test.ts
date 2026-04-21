import { describe, expect, it, vi } from 'vitest';

import { createRootRealtimeController } from './rootRealtimeController';

describe('root realtime controller', () => {
  it('dedupes repeated connect calls', async () => {
    const runtime = {
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      refreshUnreadCount: vi.fn(),
    };
    const loadRuntime = vi.fn(async () => runtime);
    const controller = createRootRealtimeController(loadRuntime);

    await Promise.all([controller.connect(), controller.connect(), controller.connect()]);

    expect(loadRuntime).toHaveBeenCalledTimes(1);
    expect(runtime.connectWebSocket).toHaveBeenCalledTimes(1);
    expect(runtime.refreshUnreadCount).toHaveBeenCalledTimes(1);
    expect(runtime.disconnectWebSocket).not.toHaveBeenCalled();
  });

  it('skips disconnect work when already disconnected', async () => {
    const runtime = {
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      refreshUnreadCount: vi.fn(),
    };
    const loadRuntime = vi.fn(async () => runtime);
    const controller = createRootRealtimeController(loadRuntime);

    await controller.disconnect();

    expect(loadRuntime).not.toHaveBeenCalled();
    expect(runtime.disconnectWebSocket).not.toHaveBeenCalled();
  });

  it('disconnects once after a connection has been established', async () => {
    const runtime = {
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      refreshUnreadCount: vi.fn(),
    };
    const loadRuntime = vi.fn(async () => runtime);
    const controller = createRootRealtimeController(loadRuntime);

    await controller.connect();
    await Promise.all([controller.disconnect(), controller.disconnect()]);

    expect(loadRuntime).toHaveBeenCalledTimes(1);
    expect(runtime.connectWebSocket).toHaveBeenCalledTimes(1);
    expect(runtime.refreshUnreadCount).toHaveBeenCalledTimes(1);
    expect(runtime.disconnectWebSocket).toHaveBeenCalledTimes(1);
  });
});

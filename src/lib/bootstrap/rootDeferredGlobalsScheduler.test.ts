import { describe, expect, it, vi } from 'vitest';

import { scheduleRootDeferredGlobals } from './rootDeferredGlobalsScheduler';

describe('root deferred globals scheduler', () => {
  it('schedules work through animation frame and idle callback when available', () => {
    const run = vi.fn();
    const requestAnimationFrame = vi.fn((cb: FrameRequestCallback) => {
      cb(16);
      return 1;
    });
    const requestIdleCallback = vi.fn((cb: IdleRequestCallback) => {
      cb({
        didTimeout: false,
        timeRemaining: () => 8,
      });
      return 2;
    });

    scheduleRootDeferredGlobals(run, {
      requestAnimationFrame,
      requestIdleCallback,
      setTimeout: vi.fn(),
    });

    expect(requestAnimationFrame).toHaveBeenCalledTimes(1);
    expect(requestIdleCallback).toHaveBeenCalledTimes(1);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it('falls back to timeout when idle callback is unavailable', () => {
    const run = vi.fn();
    const requestAnimationFrame = vi.fn((cb: FrameRequestCallback) => {
      cb(16);
      return 1;
    });
    const setTimeout = vi.fn((cb: TimerHandler) => {
      (cb as () => void)();
      return 3;
    });

    scheduleRootDeferredGlobals(run, {
      requestAnimationFrame,
      setTimeout,
    });

    expect(requestAnimationFrame).toHaveBeenCalledTimes(1);
    expect(setTimeout).toHaveBeenCalledTimes(1);
    expect(run).toHaveBeenCalledTimes(1);
  });
});

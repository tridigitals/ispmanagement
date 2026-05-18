import { describe, expect, it, vi } from 'vitest';

import { bindPopupNavigationDismiss, focusMapForPopupViewport } from './networkMapPopups';

describe('bindPopupNavigationDismiss', () => {
  it('closes the popup when map navigation starts and cleans up listeners', () => {
    const listeners = new Map<string, Set<() => void>>();
    const map = {
      on(event: string, handler: () => void) {
        if (!listeners.has(event)) listeners.set(event, new Set());
        listeners.get(event)?.add(handler);
      },
      off(event: string, handler: () => void) {
        listeners.get(event)?.delete(handler);
      },
    };
    const popup = { remove: vi.fn() };

    const cleanup = bindPopupNavigationDismiss({
      map: map as any,
      popup,
    });

    listeners.get('zoomstart')?.forEach((handler) => handler());

    expect(popup.remove).toHaveBeenCalledTimes(1);

    cleanup();

    expect(listeners.get('movestart')?.size ?? 0).toBe(0);
    expect(listeners.get('zoomstart')?.size ?? 0).toBe(0);
    expect(listeners.get('dragstart')?.size ?? 0).toBe(0);
  });
});

describe('focusMapForPopupViewport', () => {
  it('waits for moveend after easing the map before resolving', async () => {
    const listeners = new Map<string, Set<() => void>>();
    const easeTo = vi.fn();
    const map = {
      getZoom: () => 11,
      on(event: string, handler: () => void) {
        if (!listeners.has(event)) listeners.set(event, new Set());
        listeners.get(event)?.add(handler);
      },
      off(event: string, handler: () => void) {
        listeners.get(event)?.delete(handler);
      },
      easeTo(options: unknown) {
        easeTo(options);
      },
    };

    let resolved = false;
    const pending = focusMapForPopupViewport(map as any, 106.8456, -6.2088, 13).then(() => {
      resolved = true;
    });

    expect(easeTo).toHaveBeenCalledTimes(1);
    expect(resolved).toBe(false);

    listeners.get('moveend')?.forEach((handler) => handler());
    await pending;

    expect(resolved).toBe(true);
    expect(listeners.get('moveend')?.size ?? 0).toBe(0);
  });
});

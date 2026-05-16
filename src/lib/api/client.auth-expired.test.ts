import { beforeEach, describe, expect, it, vi } from 'vitest';

class MemoryStorage implements Storage {
  private map = new Map<string, string>();

  get length(): number {
    return this.map.size;
  }

  clear(): void {
    this.map.clear();
  }

  getItem(key: string): string | null {
    return this.map.has(key) ? this.map.get(key)! : null;
  }

  key(index: number): string | null {
    return Array.from(this.map.keys())[index] ?? null;
  }

  removeItem(key: string): void {
    this.map.delete(key);
  }

  setItem(key: string, value: string): void {
    this.map.set(key, String(value));
  }
}

function setupBrowser(pathname: string = '/admin/network/noc') {
  const win = new EventTarget() as any;
  const assign = vi.fn((to: string) => {
    const url = new URL(to, 'https://billing.tridigitals.com');
    win.location.pathname = url.pathname;
    win.location.search = url.search;
    win.location.hash = url.hash;
  });

  win.location = {
    protocol: 'https:',
    origin: 'https://billing.tridigitals.com',
    pathname,
    search: '',
    hash: '',
    assign,
  };

  const local = new MemoryStorage();
  const session = new MemoryStorage();

  vi.stubGlobal('window', win);
  vi.stubGlobal('localStorage', local);
  vi.stubGlobal('sessionStorage', session);
  vi.stubGlobal('navigator', { userAgent: 'Mozilla/5.0 (X11; Linux x86_64)' });

  return { win, assign, local, session };
}

describe('api client auth-expired handling', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetModules();
    vi.clearAllMocks();
  });

  it('clears auth, emits event, and redirects to login on 401 with active token', async () => {
    const { win, assign, local } = setupBrowser('/admin/network/noc');
    local.setItem('auth_token', 'token-123');
    local.setItem('auth_user', '{"id":"u1"}');
    local.setItem('auth_tenant', '{"id":"t1"}');
    local.setItem('active_tenant_slug', 'demo');

    const onExpired = vi.fn();
    win.addEventListener('app:auth-expired', onExpired);

    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ error: 'Unauthorized' }), {
          status: 401,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    const { settings } = await import('./client');

    await expect(settings.getAll()).rejects.toThrow();

    expect(local.getItem('auth_token')).toBeNull();
    expect(local.getItem('auth_user')).toBeNull();
    expect(local.getItem('auth_tenant')).toBeNull();
    expect(local.getItem('active_tenant_slug')).toBeNull();
    expect(onExpired).toHaveBeenCalledTimes(1);

    vi.runAllTimers();
    expect(assign).toHaveBeenCalledWith('/login?reason=expired');
  });

  it('does not redirect on auth errors when there is no active local token', async () => {
    const { assign } = setupBrowser('/admin/network/noc');

    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ error: 'Unauthorized' }), {
          status: 401,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    const { publicApi } = await import('./client');
    await expect(publicApi.getTenantByDomain('billing.tridigitals.com')).rejects.toThrow();

    vi.runAllTimers();
    expect(assign).not.toHaveBeenCalled();
  });

  it('does not force location.assign when already on /login', async () => {
    const { assign, local } = setupBrowser('/login');
    local.setItem('auth_token', 'token-abc');

    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ error: 'Unauthorized' }), {
          status: 401,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    const { settings } = await import('./client');
    await expect(settings.getAll()).rejects.toThrow();

    vi.runAllTimers();
    expect(assign).not.toHaveBeenCalled();
  });

  it('does not clear session or redirect on 403 forbidden', async () => {
    const { assign, local } = setupBrowser('/dashboard/services');
    local.setItem('auth_token', 'token-403');
    local.setItem('auth_user', '{"id":"u403"}');
    local.setItem('auth_tenant', '{"id":"t403"}');

    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ error: 'Forbidden' }), {
          status: 403,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    const { settings } = await import('./client');
    await expect(settings.getAll()).rejects.toThrow('Forbidden');

    expect(local.getItem('auth_token')).toBe('token-403');
    expect(local.getItem('auth_user')).toBe('{"id":"u403"}');
    expect(local.getItem('auth_tenant')).toBe('{"id":"t403"}');
    vi.runAllTimers();
    expect(assign).not.toHaveBeenCalled();
  });

  it('does not clear session or redirect on 403 even when backend message says Unauthorized', async () => {
    const { assign, local } = setupBrowser('/superadmin/storage');
    local.setItem('auth_token', 'token-403-msg');
    local.setItem('auth_user', '{"id":"u403msg","is_super_admin":true}');
    local.setItem('auth_tenant', '{"id":"t403msg"}');
    local.setItem('active_tenant_slug', 'demo');

    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ error: 'Unauthorized' }), {
          status: 403,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    const { storage } = await import('./client');
    await expect(storage.listFiles()).rejects.toThrow('Unauthorized');

    expect(local.getItem('auth_token')).toBe('token-403-msg');
    expect(local.getItem('auth_user')).toBe('{"id":"u403msg","is_super_admin":true}');
    expect(local.getItem('auth_tenant')).toBe('{"id":"t403msg"}');
    expect(local.getItem('active_tenant_slug')).toBe('demo');
    vi.runAllTimers();
    expect(assign).not.toHaveBeenCalled();
  });

  it('does not log aborted external requests as API errors', async () => {
    setupBrowser('/admin/network/map');
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    vi.stubGlobal(
      'fetch',
      vi.fn(async (_input, init) => {
        const signal = (init as RequestInit | undefined)?.signal;
        await new Promise((_, reject) => {
          if (signal?.aborted) {
            reject(new DOMException('The operation was aborted.', 'AbortError'));
            return;
          }
          signal?.addEventListener(
            'abort',
            () => reject(new DOMException('The operation was aborted.', 'AbortError')),
            { once: true },
          );
        });
        return new Response('{}', {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }),
    );

    const controller = new AbortController();
    const { safeInvoke } = await import('./core');
    const request = safeInvoke('list_network_nodes', {
      token: 'token-1',
      page: 1,
      per_page: 10,
      __signal: controller.signal,
    });
    controller.abort();

    await expect(request).rejects.toThrow('Request canceled');
    expect(consoleError).not.toHaveBeenCalled();
  });
});

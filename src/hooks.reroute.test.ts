import { afterEach, describe, expect, it, vi } from 'vitest';
import { reroute } from '../src/hooks';

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

function run(host: string, path: string) {
  return reroute({ url: new URL(`https://${host}${path}`) } as any);
}

function installBrowserTenantSlug(slug: string) {
  const local = new MemoryStorage();
  const session = new MemoryStorage();
  local.setItem('active_tenant_slug', slug);
  vi.stubGlobal('window', {} as Window & typeof globalThis);
  vi.stubGlobal('localStorage', local);
  vi.stubGlobal('sessionStorage', session);
  return { local, session };
}

describe('hooks reroute', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('leaves legacy platform path unresolved in SSR when no tenant slug is available', () => {
    expect(run('billing.tridigitals.com', '/isp-management/admin/network/noc')).toBeUndefined();
  });

  it('does not rewrite clean platform admin path', () => {
    expect(run('billing.tridigitals.com', '/admin/storage')).toBeUndefined();
  });

  it('does not rewrite platform public path', () => {
    expect(run('billing.tridigitals.com', '/login')).toBeUndefined();
  });

  it('does not rewrite superadmin storage path on platform domain', () => {
    expect(run('billing.tridigitals.com', '/superadmin/storage')).toBeUndefined();
  });

  it('does not rewrite superadmin storage path on platform domain in browser context', () => {
    installBrowserTenantSlug('xtrabit');
    expect(run('billing.tridigitals.com', '/superadmin/storage')).toBeUndefined();
  });

  it('leaves slug-prefixed platform app path unresolved in SSR when no tenant slug is available', () => {
    expect(run('billing.tridigitals.com', '/foo/admin/settings')).toBeUndefined();
  });

  it('rewrites custom domain app path to tenant slug', () => {
    expect(run('dashboard.tridigitals.com', '/admin')).toBe('/tridigitals/admin');
  });

  it('does not rewrite custom domain public path', () => {
    expect(run('dashboard.tridigitals.com', '/login')).toBeUndefined();
  });

  it('rewrites clean platform admin storage path to active tenant route in browser context', () => {
    installBrowserTenantSlug('xtrabit');
    expect(run('billing.tridigitals.com', '/admin/storage')).toBe('/xtrabit/admin/storage');
  });

  it('rewrites legacy platform storage path to active tenant route in browser context', () => {
    installBrowserTenantSlug('xtrabit');
    expect(run('billing.tridigitals.com', '/isp-management/admin/storage')).toBe(
      '/xtrabit/admin/storage',
    );
  });
});

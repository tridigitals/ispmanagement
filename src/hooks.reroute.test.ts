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

  it('rewrites legacy platform base path to clean root app path', () => {
    expect(run('billing.tridigitals.com', '/isp-management/admin/network/noc')).toBe(
      '/admin/network/noc',
    );
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

  it('does not rewrite clean custom domain app paths once root app routes exist', () => {
    expect(run('dashboard.tridigitals.com', '/admin')).toBeUndefined();
  });

  it('does not rewrite custom domain public path', () => {
    expect(run('dashboard.tridigitals.com', '/login')).toBeUndefined();
  });

  it('does not rewrite clean platform admin storage path in browser context', () => {
    installBrowserTenantSlug('xtrabit');
    expect(run('billing.tridigitals.com', '/admin/storage')).toBeUndefined();
  });

  it('does not rewrite clean localhost app paths in browser context', () => {
    installBrowserTenantSlug('demo');
    expect(run('localhost', '/admin/settings')).toBeUndefined();
  });

  it('does not rewrite clean platform admin settings path in browser context', () => {
    installBrowserTenantSlug('demo');
    expect(run('billing.tridigitals.com', '/admin/settings')).toBeUndefined();
  });

  it('keeps platform login clean even when tenant slug exists', () => {
    installBrowserTenantSlug('demo');
    expect(run('billing.tridigitals.com', '/login')).toBeUndefined();
  });

  it('does not rewrite legacy slug-prefixed platform app paths in browser context', () => {
    installBrowserTenantSlug('demo');
    expect(run('billing.tridigitals.com', '/oldslug/admin/settings')).toBeUndefined();
  });

  it('rewrites legacy base path to clean root app path in browser context', () => {
    installBrowserTenantSlug('xtrabit');
    expect(run('billing.tridigitals.com', '/isp-management/admin/storage')).toBe('/admin/storage');
  });
});

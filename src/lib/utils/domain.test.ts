import { afterEach, describe, expect, it, vi } from 'vitest';

import { cacheDomainMapping, getSlugFromDomain, isPlatformDomain } from './domain';

function createStorage() {
  const store = new Map<string, string>();
  return {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
    clear: () => {
      store.clear();
    },
  };
}

describe('domain utilities', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('treats configured billing host as platform domain', () => {
    expect(isPlatformDomain('billing.tridigitals.com')).toBe(true);
  });

  it('uses stored tenant hint on platform domain instead of guessing from hostname', () => {
    const localStorage = createStorage();
    const sessionStorage = createStorage();
    localStorage.setItem('auth_user', JSON.stringify({ tenant_slug: 'tenant-a' }));
    vi.stubGlobal('window', { location: { hostname: 'billing.tridigitals.com' } });
    vi.stubGlobal('localStorage', localStorage);
    vi.stubGlobal('sessionStorage', sessionStorage);

    expect(getSlugFromDomain('billing.tridigitals.com')).toBe('tenant-a');
    expect(getSlugFromDomain('tenant-a.billing.tridigitals.com')).toBeNull();
  });

  it('does not guess custom-domain slug without cached backend lookup', () => {
    const localStorage = createStorage();
    const sessionStorage = createStorage();
    vi.stubGlobal('window', { location: { hostname: 'portal.customer.net' } });
    vi.stubGlobal('localStorage', localStorage);
    vi.stubGlobal('sessionStorage', sessionStorage);

    expect(getSlugFromDomain('portal.customer.net')).toBeNull();
  });

  it('returns cached custom-domain mapping after successful lookup', () => {
    const localStorage = createStorage();
    const sessionStorage = createStorage();
    vi.stubGlobal('window', { location: { hostname: 'portal.customer.net' } });
    vi.stubGlobal('localStorage', localStorage);
    vi.stubGlobal('sessionStorage', sessionStorage);

    cacheDomainMapping('Portal.Customer.Net', 'tenant-b');

    expect(getSlugFromDomain('portal.customer.net')).toBe('tenant-b');
  });
});

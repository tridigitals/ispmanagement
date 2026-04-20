import { describe, expect, it } from 'vitest';

import {
  isLocalHostname,
  normalizeLegacyBasePath,
  shouldLookupCustomDomain,
} from './appBoot';

describe('app boot helpers', () => {
  it('normalizes the legacy /isp-management base path', () => {
    expect(normalizeLegacyBasePath('/isp-management', '?debug=1')).toBe('/?debug=1');
    expect(normalizeLegacyBasePath('/isp-management/login', '')).toBe('/login');
  });

  it('returns null when the path does not need legacy normalization', () => {
    expect(normalizeLegacyBasePath('/login', '?debug=1')).toBeNull();
  });

  it('detects local hostnames', () => {
    expect(isLocalHostname('localhost')).toBe(true);
    expect(isLocalHostname('127.0.0.1')).toBe(true);
    expect(isLocalHostname('my-app.tauri.local')).toBe(true);
    expect(isLocalHostname('billing.example.com')).toBe(false);
  });

  it('only looks up custom domains when host is unknown and non-platform', () => {
    expect(
      shouldLookupCustomDomain({
        hostname: 'customer.example.com',
        knownSlug: null,
        isPlatformDomain: false,
      }),
    ).toBe(true);

    expect(
      shouldLookupCustomDomain({
        hostname: 'localhost',
        knownSlug: null,
        isPlatformDomain: false,
      }),
    ).toBe(false);

    expect(
      shouldLookupCustomDomain({
        hostname: 'billing.example.com',
        knownSlug: 'tenant-a',
        isPlatformDomain: false,
      }),
    ).toBe(false);
  });
});

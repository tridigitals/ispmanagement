import { describe, expect, it } from 'vitest';
import { reroute } from '../src/hooks';

function run(host: string, path: string) {
  return reroute({ url: new URL(`https://${host}${path}`) } as any);
}

describe('hooks reroute', () => {
  it('normalizes legacy platform slug path', () => {
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

  it('normalizes slug-prefixed app path on platform domain', () => {
    expect(run('billing.tridigitals.com', '/foo/admin/settings')).toBe('/admin/settings');
  });

  it('rewrites custom domain app path to tenant slug', () => {
    expect(run('dashboard.tridigitals.com', '/admin')).toBe('/tridigitals/admin');
  });

  it('does not rewrite custom domain public path', () => {
    expect(run('dashboard.tridigitals.com', '/login')).toBeUndefined();
  });
});

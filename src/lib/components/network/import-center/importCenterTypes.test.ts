import { describe, expect, it } from 'vitest';
import { buildImportCenterSources } from './importCenterTypes';

describe('network import center helpers', () => {
  it('exposes MixRadius as the first import source under the generic import center route', () => {
    expect(buildImportCenterSources('/tenant-a')).toEqual([
      {
        key: 'mixradius',
        title: 'MixRadius',
        description: 'Migrasi backup .sql/.sql.gz MixRadius ke package, customer, subscription, dan PPPoE.',
        href: '/tenant-a/admin/network/import/mixradius',
        icon: 'download',
        status: 'ready',
        requiredPermission: { resource: 'pppoe', action: 'manage' },
      },
    ]);
  });
});

export interface ImportCenterSource {
  key: string;
  title: string;
  description: string;
  href: string;
  icon: string;
  status: 'ready' | 'coming_soon';
  requiredPermission: {
    resource: string;
    action: string;
  };
}

export function buildImportCenterSources(tenantPrefix: string): ImportCenterSource[] {
  return [
    {
      key: 'mixradius',
      title: 'MixRadius',
      description:
        'Migrasi backup .sql/.sql.gz MixRadius ke package, customer, subscription, dan PPPoE.',
      href: `${tenantPrefix}/admin/network/import/mixradius`,
      icon: 'download',
      status: 'ready',
      requiredPermission: { resource: 'pppoe', action: 'manage' },
    },
  ];
}

import { describe, expect, it } from 'vitest';

import { buildNetworkAssetConnectionItems } from './networkAssetConnections';

const rows = [
  {
    id: 'odc-1',
    asset_type: 'odc',
    name: 'ODC-1',
    status: 'available',
    parent_asset_id: null,
    parent_asset_name: null,
    customer_name: null,
    location_label: null,
  },
  {
    id: 'odp-1',
    asset_type: 'odp',
    name: 'ODP-1',
    status: 'available',
    parent_asset_id: 'odc-1',
    parent_asset_name: 'ODC-1',
    customer_name: null,
    location_label: null,
  },
  {
    id: 'ont-1',
    asset_type: 'ont',
    name: 'ONT A',
    status: 'installed',
    parent_asset_id: 'odp-1',
    parent_asset_name: 'ODP-1',
    customer_name: 'Andi',
    location_label: 'Rumah Andi',
  },
  {
    id: 'onu-1',
    asset_type: 'onu',
    name: 'ONU B',
    status: 'installed',
    parent_asset_id: 'odp-1',
    parent_asset_name: 'ODP-1',
    customer_name: 'Budi',
    location_label: 'Rumah Budi',
  },
  {
    id: 'ont-2',
    asset_type: 'ont',
    name: 'ONT C',
    status: 'installed',
    parent_asset_id: 'odp-1',
    parent_asset_name: 'ODP-1',
    customer_name: 'Citra',
    location_label: 'Rumah Citra',
  },
  {
    id: 'ont-3',
    asset_type: 'ont',
    name: 'ONT D',
    status: 'installed',
    parent_asset_id: 'odp-1',
    parent_asset_name: 'ODP-1',
    customer_name: 'Dewi',
    location_label: 'Rumah Dewi',
  },
  {
    id: 'ont-4',
    asset_type: 'ont',
    name: 'ONT E',
    status: 'installed',
    parent_asset_id: 'odp-1',
    parent_asset_name: 'ODP-1',
    customer_name: 'Eko',
    location_label: 'Rumah Eko',
  },
  {
    id: 'ont-5',
    asset_type: 'ont',
    name: 'ONT F',
    status: 'installed',
    parent_asset_id: 'odp-1',
    parent_asset_name: 'ODP-1',
    customer_name: 'Farah',
    location_label: 'Rumah Farah',
  },
] as any[];

describe('networkAssetConnections', () => {
  it('builds upstream and a capped customer-oriented summary for a distribution asset', () => {
    expect(buildNetworkAssetConnectionItems(rows[1], rows)).toEqual([
      { label: 'Upstream', value: 'ODC-1' },
      { label: 'Ports Used', value: '6 endpoint linked' },
      { label: 'Connected', value: 'Andi, Budi, Citra, Dewi, Eko +1 more' },
    ]);
  });

  it('falls back to connected customer/location info for endpoint assets', () => {
    expect(buildNetworkAssetConnectionItems(rows[2], rows)).toEqual([
      { label: 'Upstream', value: 'ODP-1' },
      { label: 'Service To', value: 'Andi • Rumah Andi' },
    ]);
  });
});

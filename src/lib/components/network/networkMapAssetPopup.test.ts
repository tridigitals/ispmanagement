import { describe, expect, it } from 'vitest';

import {
  buildTopologyAssetPopupHtml,
  escapeTopologyAssetPopupValue,
  popupToneForAssetStatus,
  type TopologyAssetPopupButtonIds,
  type TopologyAssetRow,
} from './networkMapAssets';

const BUTTON_IDS: TopologyAssetPopupButtonIds = {
  closeBtnId: 'close-1',
  connectBtnId: 'connect-1',
  editBtnId: 'edit-1',
  customerDropBtnId: 'drop-1',
};

// Identity translator: returns key so we can assert which keys were requested.
const keyTranslate = (key: string) => key;
// Empty translator: simulates a missing key so `|| fallback` paths render.
const emptyTranslate = () => '';

function makeRow(overrides: Partial<TopologyAssetRow> = {}): TopologyAssetRow {
  return {
    id: 'asset-1',
    name: 'ODP-001',
    assetType: 'odp',
    assetTypeLabel: 'ODP',
    status: 'installed',
    code: null,
    serialNumber: null,
    customerName: null,
    locationLabel: 'Jl. Merdeka',
    latitude: -6.2,
    longitude: 106.8,
    customerId: null,
    locationId: null,
    parentAssetId: null,
    markerLabel: 'ODP',
    markerColor: '#000',
    legendLabel: 'ODP',
    portCapacity: 8,
    portsUsed: 2,
    portsAvailable: 6,
    canAcceptConnections: true,
    hasUpstreamRelation: false,
    hasCustomerRelation: false,
    ...overrides,
  };
}

describe('escapeTopologyAssetPopupValue', () => {
  it('escapes HTML-significant characters', () => {
    expect(escapeTopologyAssetPopupValue('<b>"x"&\'')).toBe('&lt;b&gt;&quot;x&quot;&amp;&#039;');
  });

  it('renders a dash for nullish input', () => {
    expect(escapeTopologyAssetPopupValue(null)).toBe('-');
    expect(escapeTopologyAssetPopupValue(undefined)).toBe('-');
  });
});

describe('popupToneForAssetStatus', () => {
  it('maps installed/available to ok', () => {
    expect(popupToneForAssetStatus('installed')).toBe('ok');
    expect(popupToneForAssetStatus('available')).toBe('ok');
  });

  it('maps reserved/faulty to warn', () => {
    expect(popupToneForAssetStatus('reserved')).toBe('warn');
    expect(popupToneForAssetStatus('faulty')).toBe('warn');
  });

  it('falls back to muted for unknown status', () => {
    expect(popupToneForAssetStatus('decommissioned')).toBe('muted');
  });
});

describe('buildTopologyAssetPopupHtml', () => {
  it('wires button ids into the markup', () => {
    const html = buildTopologyAssetPopupHtml({
      row: makeRow(),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: keyTranslate,
    });
    expect(html).toContain('id="close-1"');
    expect(html).toContain('id="connect-1"');
    expect(html).toContain('id="edit-1"');
    expect(html).toContain('id="drop-1"');
  });

  it('omits the edit button when management is not permitted', () => {
    const html = buildTopologyAssetPopupHtml({
      row: makeRow(),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: false,
      translate: keyTranslate,
    });
    expect(html).not.toContain('id="edit-1"');
    expect(html).toContain('id="connect-1"');
  });

  it('renders the customer-drop relation only for odp assets', () => {
    const odp = buildTopologyAssetPopupHtml({
      row: makeRow({ assetType: 'odp' }),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: keyTranslate,
    });
    expect(odp).toContain('id="drop-1"');

    const olt = buildTopologyAssetPopupHtml({
      row: makeRow({ assetType: 'olt' }),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: keyTranslate,
    });
    expect(olt).not.toContain('id="drop-1"');
  });

  it('renders the port usage card when capacity is present', () => {
    const html = buildTopologyAssetPopupHtml({
      row: makeRow({ portCapacity: 8, portsUsed: 2, portsAvailable: 6 }),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: keyTranslate,
    });
    expect(html).toContain('nm-popup-usage-card');
    expect(html).toContain('width: 25%');
  });

  it('omits the port usage card when there is no capacity', () => {
    const html = buildTopologyAssetPopupHtml({
      row: makeRow({ portCapacity: null, portsUsed: null, portsAvailable: null }),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: keyTranslate,
    });
    expect(html).not.toContain('nm-popup-usage-card');
  });

  it('disables the connect button when the asset cannot accept connections', () => {
    const html = buildTopologyAssetPopupHtml({
      row: makeRow({ canAcceptConnections: false }),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: keyTranslate,
    });
    expect(html).toMatch(/id="connect-1"[^>]*disabled/);
  });

  it('uses fallback strings when translation keys are missing', () => {
    const html = buildTopologyAssetPopupHtml({
      row: makeRow({ portCapacity: 8, portsUsed: 8, portsAvailable: 0 }),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: emptyTranslate,
    });
    expect(html).toContain('FTTH Asset');
    expect(html).toContain('Full');
    expect(html).toContain('Close');
  });

  it('escapes asset name to prevent HTML injection', () => {
    const html = buildTopologyAssetPopupHtml({
      row: makeRow({ name: '<script>alert(1)</script>' }),
      buttonIds: BUTTON_IDS,
      canManageFtthAssets: true,
      translate: keyTranslate,
    });
    expect(html).not.toContain('<script>alert(1)</script>');
    expect(html).toContain('&lt;script&gt;');
  });
});

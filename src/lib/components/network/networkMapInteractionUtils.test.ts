import { describe, expect, it } from 'vitest';

import {
  buildLinkPopupHtml,
  buildNodePopupHtml,
  computePopupPlacement,
  computePopupViewportNudge,
  getPopupSizeForModel,
} from './networkMapInteractionUtils';

describe('computePopupViewportNudge', () => {
  it('returns zero offset when popup is already inside map viewport', () => {
    const result = computePopupViewportNudge({
      popupRect: { left: 120, right: 320, top: 120, bottom: 260 },
      mapRect: { left: 40, right: 420, top: 40, bottom: 340 },
      padding: 16,
    });

    expect(result).toEqual({ x: 0, y: 0 });
  });

  it('nudges popup back inside viewport when it overflows right and bottom edges', () => {
    const result = computePopupViewportNudge({
      popupRect: { left: 190, right: 430, top: 140, bottom: 360 },
      mapRect: { left: 40, right: 420, top: 40, bottom: 340 },
      padding: 16,
    });

    expect(result).toEqual({ x: -26, y: -36 });
  });
});

describe('computePopupPlacement', () => {
  it('prefers a left-side anchor when the point is near the right edge', () => {
    const result = computePopupPlacement({
      point: { x: 360, y: 180 },
      mapSize: { width: 400, height: 320 },
      popupSize: { width: 280, height: 220 },
      padding: 16,
    });

    expect(result.anchor).toBe('left');
    expect(result.offset).toBe(14);
  });

  it('prefers a top anchor when the point is near the top edge', () => {
    const result = computePopupPlacement({
      point: { x: 180, y: 28 },
      mapSize: { width: 400, height: 320 },
      popupSize: { width: 240, height: 180 },
      padding: 16,
    });

    expect(result.anchor).toBe('top');
    expect(result.offset).toBe(14);
  });

  it('prefers a bottom anchor when there is more room above the point than below', () => {
    const result = computePopupPlacement({
      point: { x: 220, y: 260 },
      mapSize: { width: 440, height: 420 },
      popupSize: { width: 280, height: 220 },
      padding: 16,
    });

    expect(result.anchor).toBe('bottom');
  });

  it('prefers a top anchor when there is more room below the point than above', () => {
    const result = computePopupPlacement({
      point: { x: 220, y: 120 },
      mapSize: { width: 440, height: 420 },
      popupSize: { width: 280, height: 220 },
      padding: 16,
    });

    expect(result.anchor).toBe('top');
  });
});

describe('getPopupSizeForModel', () => {
  it('uses a wider size for workflow service popups only', () => {
    expect(
      getPopupSizeForModel({
        variant: 'workflow-service',
      }),
    ).toEqual({ width: 332, height: 320 });

    expect(
      getPopupSizeForModel({
        variant: 'default',
      }),
    ).toEqual({ width: 288, height: 320 });
  });
});

describe('buildNodePopupHtml', () => {
  it('truncates long popup text for display while preserving the full value in title attributes', () => {
    const html = buildNodePopupHtml({
      popupUid: 'popup-1',
      model: {
        variant: 'workflow-service',
        kicker: 'Service',
        title: 'Home-Silver-20Mbps',
        subtitle: 'Arif Yudiyanto',
        statusText: 'active',
        tone: 'ok',
        contextText: 'Active • PPPoE • Account ready',
        summaryItems: [
          { label: 'Customer', value: 'Arif Yudiyanto Dengan Nama Sangat Panjang Sekali' },
          { label: 'Account', value: 'bandungan-duren-asepmulyana-super-panjang' },
        ],
        detailPairs: [
          { label: 'Package', value: 'Home-Silver-20Mbps-Super-Panjang-Sekali' },
          { label: 'Service', value: 'internet_pppoe' },
        ],
        actions: [],
      },
    });

    expect(html.html).toContain('title="Arif Yudiyanto Dengan Nama Sangat Panjang Sekali"');
    expect(html.html).toContain('title="bandungan-duren-asepmulyana-super-panjang"');
    expect(html.html).toContain('Arif Yudiyanto Dengan Nama...');
    expect(html.html).toContain('bandungan-duren-asepmulyana...');
    expect(html.html).toContain('Home-Silver-20Mbps-Super-Panj...');
  });
});

describe('buildLinkPopupHtml', () => {
  it('renders a compact link popup without the dense detail grid', () => {
    const html = buildLinkPopupHtml({
      popupUid: 'link-popup-1',
      model: {
        variant: 'network-link',
        kicker: 'Link',
        title: 'ODP 1 uplink',
        subtitle: 'fiber path',
        statusText: 'up',
        tone: 'ok',
        contextText: 'Fiber transport path',
        summaryItems: [
          { label: 'Health', value: '100', tone: 'ok' },
          { label: 'Capacity', value: '1 Gbps' },
        ],
        detailPairs: [
          { label: 'Endpoints', value: 'very-long-source -> very-long-target' },
        ],
        actions: [
          { key: 'edit', label: 'Edit', tone: 'primary' },
          { key: 'delete', label: 'Delete', tone: 'danger' },
        ],
      },
    });

    expect(html.html).toContain('nm-popup-card-link');
    expect(html.html).toContain('Fiber transport path');
    expect(html.html).toContain('action-edit');
    expect(html.html).toContain('Edit');
    expect(html.html).not.toContain('nm-popup-grid');
    expect(html.html).not.toContain('very-long-source');
  });
});

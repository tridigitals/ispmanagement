import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  switchComponent: { name: 'alerts-incidents-switch' },
  filterPanel: { name: 'network-filter-panel' },
  pageHeader: { name: 'network-page-header' },
  rowActions: { name: 'row-action-buttons' },
}));

vi.mock('$lib/components/network/AlertsIncidentsSwitch.svelte', () => ({
  default: sentinels.switchComponent,
}));

vi.mock('$lib/components/network/NetworkFilterPanel.svelte', () => ({
  default: sentinels.filterPanel,
}));

vi.mock('$lib/components/network/NetworkPageHeader.svelte', () => ({
  default: sentinels.pageHeader,
}));

vi.mock('$lib/components/network/RowActionButtons.svelte', () => ({
  default: sentinels.rowActions,
}));

import { loadAlertsPageModules } from './alertsPageModules';

describe('alerts page modules', () => {
  it('loads and caches heavy ui modules lazily', async () => {
    const first = await loadAlertsPageModules();
    const second = await loadAlertsPageModules();

    expect(first.AlertsIncidentsSwitchComponent).toBe(sentinels.switchComponent);
    expect(first.NetworkFilterPanelComponent).toBe(sentinels.filterPanel);
    expect(first.NetworkPageHeaderComponent).toBe(sentinels.pageHeader);
    expect(first.RowActionButtonsComponent).toBe(sentinels.rowActions);
    expect(second).toBe(first);
  });
});

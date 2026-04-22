import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  statusBanner: { name: 'system-status-banner' },
  resources: { name: 'system-resources' },
  metrics: { name: 'request-metrics' },
  statsGrid: { name: 'system-stats-grid' },
  databaseTables: { name: 'database-tables' },
  recentActivity: { name: 'recent-activity' },
  diagnosticsPanel: { name: 'system-diagnostics-panel' },
}));

vi.mock('$lib/components/superadmin/system/SystemStatusBanner.svelte', () => ({
  default: sentinels.statusBanner,
}));
vi.mock('$lib/components/superadmin/system/SystemResources.svelte', () => ({
  default: sentinels.resources,
}));
vi.mock('$lib/components/superadmin/system/RequestMetrics.svelte', () => ({
  default: sentinels.metrics,
}));
vi.mock('$lib/components/superadmin/system/SystemStatsGrid.svelte', () => ({
  default: sentinels.statsGrid,
}));
vi.mock('$lib/components/superadmin/system/DatabaseTables.svelte', () => ({
  default: sentinels.databaseTables,
}));
vi.mock('$lib/components/superadmin/system/RecentActivity.svelte', () => ({
  default: sentinels.recentActivity,
}));
vi.mock('$lib/components/superadmin/system/SystemDiagnosticsPanel.svelte', () => ({
  default: sentinels.diagnosticsPanel,
}));

import {
  loadSuperadminSystemDiagnosticsModules,
  loadSuperadminSystemHealthModules,
} from './systemPageModules';

describe('superadmin system page modules', () => {
  it('loads and caches health modules lazily', async () => {
    const first = await loadSuperadminSystemHealthModules();
    const second = await loadSuperadminSystemHealthModules();

    expect(first.SystemStatusBannerComponent).toBe(sentinels.statusBanner);
    expect(first.SystemResourcesComponent).toBe(sentinels.resources);
    expect(first.RequestMetricsComponent).toBe(sentinels.metrics);
    expect(first.SystemStatsGridComponent).toBe(sentinels.statsGrid);
    expect(first.DatabaseTablesComponent).toBe(sentinels.databaseTables);
    expect(first.RecentActivityComponent).toBe(sentinels.recentActivity);
    expect(second).toBe(first);
  });

  it('loads and caches diagnostics modules lazily', async () => {
    const first = await loadSuperadminSystemDiagnosticsModules();
    const second = await loadSuperadminSystemDiagnosticsModules();

    expect(first.SystemDiagnosticsPanelComponent).toBe(sentinels.diagnosticsPanel);
    expect(second).toBe(first);
  });
});

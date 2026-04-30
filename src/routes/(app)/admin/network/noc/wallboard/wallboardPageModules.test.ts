import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  alertsPanel: { name: 'wallboard-alerts-panel' },
  insightsShell: { name: 'wallboard-insights-shell' },
  insightsControls: { name: 'wallboard-insights-controls' },
  insightsSummary: { name: 'wallboard-insights-summary' },
  slotPicker: { name: 'wallboard-slot-picker' },
  fullDialog: { name: 'wallboard-full-dialog' },
  thresholdDialog: { name: 'wallboard-threshold-dialog' },
}));

vi.mock('$lib/components/network/WallboardAlertsPanel.svelte', () => ({
  default: sentinels.alertsPanel,
}));

vi.mock('./WallboardInsightsShell.svelte', () => ({
  default: sentinels.insightsShell,
}));

vi.mock('$lib/components/network/WallboardInsightsControls.svelte', () => ({
  default: sentinels.insightsControls,
}));

vi.mock('$lib/components/network/WallboardInsightsSummary.svelte', () => ({
  default: sentinels.insightsSummary,
}));

vi.mock('$lib/components/network/WallboardSlotPicker.svelte', () => ({
  default: sentinels.slotPicker,
}));

vi.mock('$lib/components/network/WallboardFullDialog.svelte', () => ({
  default: sentinels.fullDialog,
}));

vi.mock('$lib/components/network/WallboardThresholdDialog.svelte', () => ({
  default: sentinels.thresholdDialog,
}));

import {
  loadWallboardAlertsPanel,
  loadWallboardDialogs,
  loadWallboardExportModule,
  loadWallboardInsightsShell,
  loadWallboardInsightsModules,
} from './wallboardPageModules';

describe('wallboard page modules', () => {
  it('loads and caches the wallboard insights modules lazily', async () => {
    const first = await loadWallboardInsightsModules();
    const second = await loadWallboardInsightsModules();

    expect(first.InsightsControlsComponent).toBe(sentinels.insightsControls);
    expect(first.InsightsSummaryComponent).toBe(sentinels.insightsSummary);
    expect(second).toBe(first);
  });

  it('loads and caches the wallboard alerts panel lazily', async () => {
    const first = await loadWallboardAlertsPanel();
    const second = await loadWallboardAlertsPanel();

    expect(first.AlertsPanelComponent).toBe(sentinels.alertsPanel);
    expect(second).toBe(first);
  });

  it('loads and caches the wallboard insights shell lazily', async () => {
    const first = await loadWallboardInsightsShell();
    const second = await loadWallboardInsightsShell();

    expect(first.InsightsShellComponent).toBe(sentinels.insightsShell);
    expect(second).toBe(first);
  });

  it('loads and caches the wallboard dialogs lazily', async () => {
    const first = await loadWallboardDialogs();
    const second = await loadWallboardDialogs();

    expect(first.SlotPickerComponent).toBe(sentinels.slotPicker);
    expect(first.FullDialogComponent).toBe(sentinels.fullDialog);
    expect(first.ThresholdDialogComponent).toBe(sentinels.thresholdDialog);
    expect(second).toBe(first);
  });

  it('loads and caches the export helpers lazily', async () => {
    const first = await loadWallboardExportModule();
    const second = await loadWallboardExportModule();

    expect(typeof first.exportCsvRows).toBe('function');
    expect(second).toBe(first);
  });
});

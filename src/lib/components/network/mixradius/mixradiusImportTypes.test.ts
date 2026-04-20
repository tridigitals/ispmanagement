import { describe, expect, it } from 'vitest';
import {
  buildMixradiusBatchHistory,
  buildMixradiusBatchReport,
  buildMixradiusExecutionHighlights,
  buildMixradiusPreviewCounts,
  buildMixradiusSourceSummaryCards,
  getMixradiusBatchStatusMeta,
  getMixradiusConflictBadge,
  getMixradiusExecutionModeLabel,
  getMixradiusLifecycleLabel,
  getMixradiusSafeModeExecuteState,
  resolveMixradiusResumeStep,
  type MixradiusImportBatch,
  type MixradiusImportExecutionResult,
  type MixradiusImportPreviewRow,
} from './mixradiusImportTypes';

const row = (
  conflictState: MixradiusImportPreviewRow['conflictState'],
  sourceKind = 'customer'
): MixradiusImportPreviewRow => ({
  rowNumber: 1,
  sourceKind,
  sourceRef: `${sourceKind}-1`,
  targetKind: null,
  targetId: null,
  displayName: `${sourceKind} row`,
  conflictState,
  notes: null,
});

const batch = (summaryJson: Record<string, unknown>): MixradiusImportBatch => ({
  id: 'batch-1',
  tenantId: 'tenant-1',
  sourceFilename: 'MixRadius.sql.gz',
  sourceSha256: 'sha256',
  sourceSizeBytes: 1024,
  parseStatus: 'ready',
  executionStatus: 'completed',
  executionMode: 'safe_import',
  progressJson: {},
  summaryJson,
  errorJson: [],
  createdAt: '2026-04-11T00:00:00Z',
  updatedAt: '2026-04-11T00:00:00Z',
});

const executionResult = (
  summaryJson: Record<string, unknown>,
  warnings: string[] = []
): MixradiusImportExecutionResult => ({
  batch: batch(summaryJson),
  summary: {
    batchId: 'batch-1',
    mode: 'safe_import',
    totalRows: 557,
    importedRows: 105,
    updatedRows: 8,
    skippedRows: 3,
    blockedRows: 0,
    conflictRows: 2,
    warnings,
  },
  preview: null,
  warnings,
});

describe('mixradius import UI helpers', () => {
  it('maps conflict states into badge labels and tones', () => {
    expect(getMixradiusConflictBadge('auto_matched')).toEqual({
      label: 'Auto matched',
      tone: 'success',
    });
    expect(getMixradiusConflictBadge('needs_review')).toEqual({
      label: 'Needs review',
      tone: 'warning',
    });
    expect(getMixradiusConflictBadge('blocked')).toEqual({
      label: 'Blocked',
      tone: 'danger',
    });
  });

  it('maps lifecycle statuses into operator-friendly labels', () => {
    expect(getMixradiusLifecycleLabel('active')).toBe('Aktif');
    expect(getMixradiusLifecycleLabel('pending_installation')).toBe('Menunggu instalasi');
    expect(getMixradiusLifecycleLabel('installation_done_awaiting_payment')).toBe(
      'Instalasi selesai, menunggu pembayaran'
    );
    expect(getMixradiusLifecycleLabel('unknown_legacy_state')).toBe('unknown legacy state');
  });

  it('formats preview counts for summary cards', () => {
    expect(
      buildMixradiusPreviewCounts([
        row('auto_matched', 'plan'),
        row('needs_review', 'customer'),
        row('conflict', 'customer'),
        row('blocked', 'nas'),
        row('skipped', 'usage'),
      ])
    ).toEqual({
      total: 5,
      autoMatched: 1,
      needsReview: 1,
      conflicts: 1,
      blocked: 1,
      skipped: 1,
      bySourceKind: {
        customer: 2,
        nas: 1,
        plan: 1,
        usage: 1,
      },
    });
  });

  it('builds source summary cards with PPP, unsupported hotspot, accounting, and location coverage', () => {
    expect(
      buildMixradiusSourceSummaryCards(
        batch({
          customersTotal: 545,
          customersPpp: 543,
          plansPpp: 12,
          nas: 2,
          transactions: 1902,
          radacct: 3811,
          customerLocations: 460,
          usageRows: 0,
        })
      )
    ).toEqual([
      { key: 'customersPpp', label: 'Customers PPP', value: 543, icon: 'users' },
      {
        key: 'customersUnsupported',
        label: 'Unsupported hotspot',
        value: 2,
        icon: 'alert-triangle',
      },
      { key: 'plansPpp', label: 'Plans PPP', value: 12, icon: 'package' },
      { key: 'nas', label: 'NAS', value: 2, icon: 'router' },
      { key: 'transactions', label: 'Transactions', value: 1902, icon: 'receipt' },
      { key: 'radacct', label: 'Accounting rows', value: 3811, icon: 'activity' },
      { key: 'customerLocations', label: 'Location rows', value: 460, icon: 'map-pin' },
      { key: 'usageRows', label: 'Usage rows', value: 0, icon: 'activity' },
    ]);
  });

  it('builds execution highlights from persisted batch summary for lifecycle-aware reporting', () => {
    expect(
      buildMixradiusExecutionHighlights(
        executionResult(
          {
            phaseReports: {
              packages: { importedRows: 12, updatedRows: 1 },
              customers: {
                importedRows: 20,
                updatedRows: 3,
                locationImportedRows: 18,
                locationUpdatedRows: 4,
              },
              subscriptions: { importedRows: 19, updatedRows: 2 },
              pppoe: { importedRows: 17, updatedRows: 5 },
            },
            legacyTransactionCount: 1902,
            productionInvoiceCount: 0,
          },
          ['Billing invoice produksi masih tahap berikutnya.']
        )
      )
    ).toEqual({
      totals: [
        { key: 'importedRows', label: 'Imported', value: 105 },
        { key: 'updatedRows', label: 'Updated', value: 8 },
        { key: 'conflictRows', label: 'Conflicts', value: 2 },
        { key: 'blockedRows', label: 'Blocked', value: 0 },
      ],
      phases: [
        { key: 'packages', label: 'Packages', imported: 12, updated: 1 },
        { key: 'customers', label: 'Customers', imported: 20, updated: 3 },
        { key: 'locations', label: 'Locations', imported: 18, updated: 4 },
        { key: 'subscriptions', label: 'Subscriptions', imported: 19, updated: 2 },
        { key: 'pppoe', label: 'PPPoE', imported: 17, updated: 5 },
      ],
      billing: {
        legacyTransactionCount: 1902,
        productionInvoiceCount: 0,
        warnings: ['Billing invoice produksi masih tahap berikutnya.'],
      },
    });
  });

  it('maps batch parse/execution status into operator-friendly labels', () => {
    expect(getMixradiusBatchStatusMeta(batch({}))).toEqual({
      label: 'Completed',
      tone: 'success',
    });
    expect(
      getMixradiusBatchStatusMeta({
        ...batch({}),
        parseStatus: 'failed',
        executionStatus: 'pending',
      })
    ).toEqual({
      label: 'Parse failed',
      tone: 'danger',
    });
    expect(
      getMixradiusBatchStatusMeta({
        ...batch({}),
        parseStatus: 'ready',
        executionStatus: 'partial_success',
      })
    ).toEqual({
      label: 'Partial success',
      tone: 'warning',
    });
  });

  it('builds batch history ordered by newest update with summary counts', () => {
    expect(
      buildMixradiusBatchHistory([
        {
          ...batch({ customersPpp: 10 }),
          id: 'older',
          sourceFilename: 'mixradius_import_older-uuid_older.sql.gz',
          updatedAt: '2026-04-10T10:00:00Z',
        },
        {
          ...batch({ customersPpp: 543 }),
          id: 'newer',
          sourceFilename:
            'mixradius_import_newer-uuid_MixRadiusDB_Gasal_2026-04-13_085944.sql',
          updatedAt: '2026-04-11T10:00:00Z',
          executionStatus: 'running',
        },
      ])
    ).toEqual([
      {
        id: 'newer',
        title: 'MixRadiusDB_Gasal_2026-04-13_085944.sql',
        customerCount: 543,
        updatedAt: '2026-04-11T10:00:00Z',
        status: { label: 'Running', tone: 'warning' },
      },
      {
        id: 'older',
        title: 'older.sql.gz',
        customerCount: 10,
        updatedAt: '2026-04-10T10:00:00Z',
        status: { label: 'Completed', tone: 'success' },
      },
    ]);
  });

  it('builds a read-only batch report from persisted summary json', () => {
    expect(
      buildMixradiusBatchReport(
        batch({
          customersPpp: 543,
          plansPpp: 12,
          nas: 2,
          transactions: 1902,
          phaseReports: {
            packages: { status: 'completed', importedRows: 12, updatedRows: 0 },
            customers: {
              status: 'completed',
              importedRows: 20,
              updatedRows: 3,
              locationImportedRows: 18,
              locationUpdatedRows: 4,
            },
            subscriptions: { status: 'completed', importedRows: 19, updatedRows: 2 },
            pppoe: { status: 'failed', importedRows: 0, updatedRows: 0 },
          },
          legacyTransactionCount: 1902,
          productionInvoiceCount: 0,
          errors: [{ phase: 'pppoe', message: 'pppoe_accounts missing' }],
        })
      )
    ).toEqual({
      status: { label: 'Completed', tone: 'success' },
      source: [
        { key: 'customersPpp', label: 'Customers PPP', value: 543, icon: 'users' },
        { key: 'plansPpp', label: 'Plans PPP', value: 12, icon: 'package' },
        { key: 'nas', label: 'NAS', value: 2, icon: 'router' },
        { key: 'transactions', label: 'Transactions', value: 1902, icon: 'receipt' },
      ],
      phases: [
        { key: 'packages', label: 'Packages', status: 'completed', imported: 12, updated: 0 },
        { key: 'customers', label: 'Customers', status: 'completed', imported: 20, updated: 3 },
        { key: 'locations', label: 'Locations', status: 'completed', imported: 18, updated: 4 },
        {
          key: 'subscriptions',
          label: 'Subscriptions',
          status: 'completed',
          imported: 19,
          updated: 2,
        },
        { key: 'pppoe', label: 'PPPoE', status: 'failed', imported: 0, updated: 0 },
      ],
      billing: {
        legacyTransactionCount: 1902,
        productionInvoiceCount: 0,
      },
      errors: [{ phase: 'pppoe', message: 'pppoe_accounts missing' }],
    });
  });

  it('maps execution mode labels and descriptions', () => {
    expect(getMixradiusExecutionModeLabel('preview_only')).toEqual({
      label: 'Preview only',
      description: 'Simulasi import tanpa menulis data produksi.',
    });
    expect(getMixradiusExecutionModeLabel('safe_import').label).toBe('Safe import');
    expect(getMixradiusExecutionModeLabel('force_sync').label).toBe('Force sync');
  });

  it('blocks safe mode execution while unresolved blocked or conflict rows remain', () => {
    expect(getMixradiusSafeModeExecuteState('safe_import', [row('blocked')])).toEqual({
      disabled: true,
      reason: 'Selesaikan blocked/conflict rows sebelum menjalankan safe import.',
    });
    expect(getMixradiusSafeModeExecuteState('safe_import', [row('conflict')]).disabled).toBe(true);
    expect(getMixradiusSafeModeExecuteState('safe_import', [row('needs_review')])).toEqual({
      disabled: false,
      reason: null,
    });
    expect(getMixradiusSafeModeExecuteState('force_sync', [row('blocked')])).toEqual({
      disabled: false,
      reason: null,
    });
  });

  it('resumes incomplete staged batches from mapping before jumping to preview', () => {
    expect(
      resolveMixradiusResumeStep({
        parseStatus: 'ready',
        executionStatus: 'pending',
        hasPreviewRequest: true,
        hasExecuteRequest: false,
      })
    ).toBe('mapping');

    expect(
      resolveMixradiusResumeStep({
        parseStatus: 'ready',
        executionStatus: 'completed',
        hasPreviewRequest: true,
        hasExecuteRequest: true,
      })
    ).toBe('execute');

    expect(
      resolveMixradiusResumeStep({
        parseStatus: 'running',
        executionStatus: 'pending',
        hasPreviewRequest: false,
        hasExecuteRequest: false,
      })
    ).toBe('source');
  });
});

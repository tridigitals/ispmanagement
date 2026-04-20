export type MixradiusImportConflictState =
  | 'auto_matched'
  | 'needs_review'
  | 'conflict'
  | 'blocked'
  | 'skipped';

export type MixradiusImportConflictResolution = 'merge' | 'create_new' | 'skip';
export type MixradiusImportLocationStrategy = 'preserve' | 'merge' | 'replace';
export type MixradiusImportExecutionMode = 'preview_only' | 'safe_import' | 'force_sync';
export type MixradiusImportPppoeProvisioningTarget = 'router' | 'managed_radius';

export interface MixradiusImportMappingOverrideInput {
  source_kind: string;
  source_value: string;
  target_kind: string;
  target_value: string;
}

export interface MixradiusImportBatch {
  id: string;
  tenantId: string;
  sourceFilename: string;
  sourceSha256: string;
  sourceSizeBytes: number;
  parseStatus: string;
  executionStatus: string;
  executionMode: MixradiusImportExecutionMode;
  startedAt?: string | null;
  completedAt?: string | null;
  progressJson: Record<string, unknown>;
  summaryJson: Record<string, unknown>;
  errorJson: unknown[];
  createdBy?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface MixradiusImportPreviewRow {
  rowNumber: number;
  sourceKind: string;
  sourceRef: string;
  targetKind: string | null;
  targetId: string | null;
  displayName: string | null;
  conflictState: MixradiusImportConflictState;
  notes: string | null;
}

export interface MixradiusImportPreview {
  batchId: string;
  totalRows: number;
  rows: MixradiusImportPreviewRow[];
  generatedAt: string;
}

export interface MixradiusImportExecutionSummary {
  batchId: string;
  mode: MixradiusImportExecutionMode;
  totalRows: number;
  importedRows: number;
  updatedRows: number;
  skippedRows: number;
  blockedRows: number;
  conflictRows: number;
  warnings: string[];
}

export interface MixradiusImportExecutionResult {
  batch: MixradiusImportBatch;
  summary: MixradiusImportExecutionSummary;
  preview?: MixradiusImportPreview | null;
  warnings: string[];
}

export interface MixradiusSourceSummaryCard {
  key: string;
  label: string;
  value: number;
  icon: string;
}

export interface MixradiusExecutionHighlightRow {
  key: string;
  label: string;
  value: number;
}

export interface MixradiusExecutionPhaseHighlight {
  key: string;
  label: string;
  imported: number;
  updated: number;
}

export interface MixradiusExecutionHighlights {
  totals: MixradiusExecutionHighlightRow[];
  phases: MixradiusExecutionPhaseHighlight[];
  billing: {
    legacyTransactionCount: number;
    productionInvoiceCount: number;
    warnings: string[];
  };
}

export interface MixradiusConflictBadgeMeta {
  label: string;
  tone: 'success' | 'warning' | 'danger' | 'muted';
}

export interface MixradiusExecutionModeMeta {
  label: string;
  description: string;
}

export interface MixradiusProvisioningTargetMeta {
  label: string;
  description: string;
}

export interface MixradiusBatchStatusMeta {
  label: string;
  tone: 'success' | 'warning' | 'danger' | 'muted';
}

export interface MixradiusBatchHistoryItem {
  id: string;
  title: string;
  customerCount: number;
  updatedAt: string;
  status: MixradiusBatchStatusMeta;
}

export interface MixradiusBatchReportPhase {
  key: string;
  label: string;
  status: string;
  imported: number;
  updated: number;
}

export interface MixradiusBatchReport {
  status: MixradiusBatchStatusMeta;
  source: MixradiusSourceSummaryCard[];
  phases: MixradiusBatchReportPhase[];
  billing: {
    legacyTransactionCount: number;
    productionInvoiceCount: number;
  };
  errors: Array<Record<string, unknown>>;
}

export interface MixradiusPreviewCounts {
  total: number;
  autoMatched: number;
  needsReview: number;
  conflicts: number;
  blocked: number;
  skipped: number;
  bySourceKind: Record<string, number>;
}

export interface MixradiusExecuteState {
  disabled: boolean;
  reason: string | null;
}

export type MixradiusWizardStep = 'upload' | 'source' | 'mapping' | 'preview' | 'execute';

const CONFLICT_BADGE_META: Record<MixradiusImportConflictState, MixradiusConflictBadgeMeta> = {
  auto_matched: { label: 'Auto matched', tone: 'success' },
  needs_review: { label: 'Needs review', tone: 'warning' },
  conflict: { label: 'Conflict', tone: 'danger' },
  blocked: { label: 'Blocked', tone: 'danger' },
  skipped: { label: 'Skipped', tone: 'muted' },
};

const EXECUTION_MODE_META: Record<MixradiusImportExecutionMode, MixradiusExecutionModeMeta> = {
  preview_only: {
    label: 'Preview only',
    description: 'Simulasi import tanpa menulis data produksi.',
  },
  safe_import: {
    label: 'Safe import',
    description: 'Import aman dengan skip conflict dan tanpa overwrite agresif.',
  },
  force_sync: {
    label: 'Force sync',
    description: 'Paksa sinkronisasi field yang diizinkan saat conflict tertentu.',
  },
};

const PROVISIONING_TARGET_META: Record<
  MixradiusImportPppoeProvisioningTarget,
  MixradiusProvisioningTargetMeta
> = {
  router: {
    label: 'Router local secret',
    description: 'Buat akun PPPoE sebagai secret lokal di router terpilih.',
  },
  managed_radius: {
    label: 'Managed RADIUS',
    description:
      'Simpan akun sebagai Managed RADIUS. Server RADIUS mengikuti assignment aktif pada router yang dipilih.',
  },
};

const LIFECYCLE_LABELS: Record<string, string> = {
  active: 'Aktif',
  grace_active: 'Masa tenggang aktif',
  pending_installation: 'Menunggu instalasi',
  installation_done_awaiting_payment: 'Instalasi selesai, menunggu pembayaran',
  suspended: 'Suspended',
  cancelled: 'Dibatalkan',
};

const EXECUTION_STATUS_META: Record<string, MixradiusBatchStatusMeta> = {
  pending: { label: 'Pending', tone: 'muted' },
  running: { label: 'Running', tone: 'warning' },
  completed: { label: 'Completed', tone: 'success' },
  partial_success: { label: 'Partial success', tone: 'warning' },
  failed: { label: 'Failed', tone: 'danger' },
  cancelled: { label: 'Cancelled', tone: 'muted' },
};

export function getMixradiusConflictBadge(
  state: MixradiusImportConflictState
): MixradiusConflictBadgeMeta {
  return CONFLICT_BADGE_META[state];
}

export function getMixradiusLifecycleLabel(status: string): string {
  return LIFECYCLE_LABELS[status] ?? status.replaceAll('_', ' ');
}

export function getMixradiusExecutionModeLabel(
  mode: MixradiusImportExecutionMode
): MixradiusExecutionModeMeta {
  return EXECUTION_MODE_META[mode];
}

export function getMixradiusProvisioningTargetLabel(
  target: MixradiusImportPppoeProvisioningTarget
): MixradiusProvisioningTargetMeta {
  return PROVISIONING_TARGET_META[target];
}

export function buildMixradiusPreviewCounts(
  rows: MixradiusImportPreviewRow[]
): MixradiusPreviewCounts {
  const counts: MixradiusPreviewCounts = {
    total: rows.length,
    autoMatched: 0,
    needsReview: 0,
    conflicts: 0,
    blocked: 0,
    skipped: 0,
    bySourceKind: {},
  };

  for (const row of rows) {
    if (row.conflictState === 'auto_matched') counts.autoMatched += 1;
    if (row.conflictState === 'needs_review') counts.needsReview += 1;
    if (row.conflictState === 'conflict') counts.conflicts += 1;
    if (row.conflictState === 'blocked') counts.blocked += 1;
    if (row.conflictState === 'skipped') counts.skipped += 1;
    counts.bySourceKind[row.sourceKind] = (counts.bySourceKind[row.sourceKind] ?? 0) + 1;
  }

  return counts;
}

export function getMixradiusSafeModeExecuteState(
  mode: MixradiusImportExecutionMode,
  rows: MixradiusImportPreviewRow[]
): MixradiusExecuteState {
  if (mode !== 'safe_import') {
    return { disabled: false, reason: null };
  }

  const hasUnsafeRows = rows.some(
    (row) => row.conflictState === 'blocked' || row.conflictState === 'conflict'
  );
  if (!hasUnsafeRows) {
    return { disabled: false, reason: null };
  }

  return {
    disabled: true,
    reason: 'Selesaikan blocked/conflict rows sebelum menjalankan safe import.',
  };
}

function readNumber(value: unknown): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string') {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) return parsed;
  }
  return 0;
}

function readRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

export function buildMixradiusSourceSummaryCards(
  batch: MixradiusImportBatch | null
): MixradiusSourceSummaryCard[] {
  const summary = readRecord(batch?.summaryJson);
  const customersTotal = readNumber(summary.customersTotal);
  const customersPpp = readNumber(summary.customersPpp);

  return [
    { key: 'customersPpp', label: 'Customers PPP', value: customersPpp, icon: 'users' },
    {
      key: 'customersUnsupported',
      label: 'Unsupported hotspot',
      value: Math.max(customersTotal - customersPpp, 0),
      icon: 'alert-triangle',
    },
    { key: 'plansPpp', label: 'Plans PPP', value: readNumber(summary.plansPpp), icon: 'package' },
    { key: 'nas', label: 'NAS', value: readNumber(summary.nas), icon: 'router' },
    {
      key: 'transactions',
      label: 'Transactions',
      value: readNumber(summary.transactions),
      icon: 'receipt',
    },
    {
      key: 'radacct',
      label: 'Accounting rows',
      value: readNumber(summary.radacct),
      icon: 'activity',
    },
    {
      key: 'customerLocations',
      label: 'Location rows',
      value: readNumber(summary.customerLocations),
      icon: 'map-pin',
    },
    {
      key: 'usageRows',
      label: 'Usage rows',
      value: readNumber(summary.usageRows),
      icon: 'activity',
    },
  ];
}

export function buildMixradiusExecutionHighlights(
  result: MixradiusImportExecutionResult | null
): MixradiusExecutionHighlights {
  const summary = readRecord(result?.batch?.summaryJson);
  const phaseReports = readRecord(summary.phaseReports);
  const customerPhase = readRecord(phaseReports.customers);

  return {
    totals: [
      { key: 'importedRows', label: 'Imported', value: readNumber(result?.summary.importedRows) },
      { key: 'updatedRows', label: 'Updated', value: readNumber(result?.summary.updatedRows) },
      { key: 'conflictRows', label: 'Conflicts', value: readNumber(result?.summary.conflictRows) },
      { key: 'blockedRows', label: 'Blocked', value: readNumber(result?.summary.blockedRows) },
    ],
    phases: [
      {
        key: 'packages',
        label: 'Packages',
        imported: readNumber(readRecord(phaseReports.packages).importedRows),
        updated: readNumber(readRecord(phaseReports.packages).updatedRows),
      },
      {
        key: 'customers',
        label: 'Customers',
        imported: readNumber(customerPhase.importedRows),
        updated: readNumber(customerPhase.updatedRows),
      },
      {
        key: 'locations',
        label: 'Locations',
        imported: readNumber(customerPhase.locationImportedRows),
        updated: readNumber(customerPhase.locationUpdatedRows),
      },
      {
        key: 'subscriptions',
        label: 'Subscriptions',
        imported: readNumber(readRecord(phaseReports.subscriptions).importedRows),
        updated: readNumber(readRecord(phaseReports.subscriptions).updatedRows),
      },
      {
        key: 'pppoe',
        label: 'PPPoE',
        imported: readNumber(readRecord(phaseReports.pppoe).importedRows),
        updated: readNumber(readRecord(phaseReports.pppoe).updatedRows),
      },
    ],
    billing: {
      legacyTransactionCount: readNumber(summary.legacyTransactionCount),
      productionInvoiceCount: readNumber(summary.productionInvoiceCount),
      warnings: result?.summary.warnings ?? [],
    },
  };
}

export function getMixradiusBatchStatusMeta(
  batch: MixradiusImportBatch
): MixradiusBatchStatusMeta {
  if (batch.parseStatus === 'failed') {
    return { label: 'Parse failed', tone: 'danger' };
  }

  return EXECUTION_STATUS_META[batch.executionStatus] ?? {
    label: batch.executionStatus.replaceAll('_', ' '),
    tone: 'muted',
  };
}

export function buildMixradiusBatchHistory(
  batches: MixradiusImportBatch[]
): MixradiusBatchHistoryItem[] {
  return [...batches]
    .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
    .map((batch) => ({
      id: batch.id,
      title: formatMixradiusBatchHistoryTitle(batch.sourceFilename),
      customerCount: readNumber(readRecord(batch.summaryJson).customersPpp),
      updatedAt: batch.updatedAt,
      status: getMixradiusBatchStatusMeta(batch),
    }));
}

export function resolveMixradiusResumeStep(input: {
  parseStatus: string;
  executionStatus: string;
  hasPreviewRequest: boolean;
  hasExecuteRequest: boolean;
}): MixradiusWizardStep {
  if (input.parseStatus !== 'ready') return 'source';

  if (input.executionStatus === 'completed' || input.executionStatus === 'partial_success') {
    return 'execute';
  }

  if (input.hasExecuteRequest) {
    return 'execute';
  }

  if (input.hasPreviewRequest) {
    return 'mapping';
  }

  return 'source';
}

function formatMixradiusBatchHistoryTitle(sourceFilename: string): string {
  const trimmed = String(sourceFilename || '').trim();
  if (!trimmed) return 'MixRadius backup';

  const mixradiusMarker = trimmed.lastIndexOf('MixRadiusDB_');
  if (mixradiusMarker >= 0) {
    return trimmed.slice(mixradiusMarker);
  }

  const prefixed = trimmed.replace(/^mixradius_import_[^_]+_/, '');
  return prefixed || trimmed;
}

export function buildMixradiusBatchReport(batch: MixradiusImportBatch | null): MixradiusBatchReport {
  const summary = readRecord(batch?.summaryJson);
  const phaseReports = readRecord(summary.phaseReports);
  const customerPhase = readRecord(phaseReports.customers);
  const source = buildMixradiusSourceSummaryCards(batch).filter((card) =>
    ['customersPpp', 'plansPpp', 'nas', 'transactions'].includes(card.key)
  );
  const errors = Array.isArray(summary.errors)
    ? (summary.errors as Array<Record<string, unknown>>)
    : [];

  return {
    status: batch ? getMixradiusBatchStatusMeta(batch) : { label: 'Unknown', tone: 'muted' },
    source,
    phases: [
      buildReportPhase('packages', 'Packages', readRecord(phaseReports.packages)),
      buildReportPhase('customers', 'Customers', customerPhase),
      {
        key: 'locations',
        label: 'Locations',
        status: String(customerPhase.status ?? 'pending'),
        imported: readNumber(customerPhase.locationImportedRows),
        updated: readNumber(customerPhase.locationUpdatedRows),
      },
      buildReportPhase('subscriptions', 'Subscriptions', readRecord(phaseReports.subscriptions)),
      buildReportPhase('pppoe', 'PPPoE', readRecord(phaseReports.pppoe)),
    ],
    billing: {
      legacyTransactionCount: readNumber(summary.legacyTransactionCount ?? summary.transactions),
      productionInvoiceCount: readNumber(summary.productionInvoiceCount),
    },
    errors,
  };
}

function buildReportPhase(
  key: string,
  label: string,
  phase: Record<string, unknown>
): MixradiusBatchReportPhase {
  return {
    key,
    label,
    status: String(phase.status ?? 'pending'),
    imported: readNumber(phase.importedRows),
    updated: readNumber(phase.updatedRows),
  };
}

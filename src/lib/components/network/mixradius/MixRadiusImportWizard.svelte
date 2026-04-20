<script lang="ts">
  // Manual QA checklist:
  // 1. Route /{tenant}/admin/network/import/mixradius opens from the network import center.
  // 2. Upload step accepts .sql / .sql.gz via Tauri file picker and blocks next step on failure.
  // 3. Mapping step persists NAS/package overrides and customer/location strategies into preview requests.
  // 4. Preview step shows counts and tabs for blocked/conflict/review states.
  // 5. Execute step supports preview_only, safe_import, and force_sync plus cancel action.
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { api } from '$lib/api/client';
  import { isTauriRuntime } from '$lib/api/core';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import MixRadiusBatchReportDrawer from './MixRadiusBatchReportDrawer.svelte';
  import MixRadiusExecutionStep from './MixRadiusExecutionStep.svelte';
  import MixRadiusMappingStep from './MixRadiusMappingStep.svelte';
  import MixRadiusPreviewStep from './MixRadiusPreviewStep.svelte';
  import MixRadiusSourceSummaryStep from './MixRadiusSourceSummaryStep.svelte';
  import MixRadiusUploadStep from './MixRadiusUploadStep.svelte';
  import type {
    MixradiusImportBatch,
    MixradiusBatchHistoryItem,
    MixradiusImportConflictResolution,
    MixradiusImportExecutionMode,
    MixradiusImportExecutionResult,
    MixradiusImportLocationStrategy,
    MixradiusImportMappingOverrideInput,
    MixradiusImportPppoeProvisioningTarget,
    MixradiusImportPreview,
    MixradiusImportPreviewRow,
  } from './mixradiusImportTypes';
  import { buildMixradiusBatchHistory, resolveMixradiusResumeStep } from './mixradiusImportTypes';

  type Option = { label: string; value: string };
  type WizardStep = 'upload' | 'source' | 'mapping' | 'preview' | 'execute';

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      routeTenantSlug: $page.params.tenant,
    })
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);

  let step = $state<WizardStep>('upload');
  let loading = $state(false);
  let uploading = $state(false);
  let executing = $state(false);
  let error = $state('');

  let selectedPath = $state('');
  let selectedName = $state('');
  let selectedSize = $state(0);
  let selectedFile = $state<File | null>(null);

  let batch = $state<MixradiusImportBatch | null>(null);
  let initialPreview = $state<MixradiusImportPreview | null>(null);
  let preview = $state<MixradiusImportPreview | null>(null);
  let executionResult = $state<MixradiusImportExecutionResult | null>(null);

  let mappingOverrides = $state<MixradiusImportMappingOverrideInput[]>([]);
  let customerConflictResolution = $state<MixradiusImportConflictResolution | null>(null);
  let locationStrategy = $state<MixradiusImportLocationStrategy | null>(null);
  let pppoeProvisioningTarget =
    $state<MixradiusImportPppoeProvisioningTarget>('managed_radius');
  let executionMode = $state<MixradiusImportExecutionMode>('safe_import');
  let previewTab = $state<'all' | MixradiusImportPreviewRow['conflictState']>('all');
  let batchHistory = $state<MixradiusBatchHistoryItem[]>([]);
  let historyLoading = $state(false);
  let reportOpen = $state(false);
  let reportLoading = $state(false);
  let reportBatch = $state<MixradiusImportBatch | null>(null);

  let routerOptions = $state<Option[]>([]);
  let packageOptions = $state<Option[]>([]);
  const isDesktopApp = isTauriRuntime();

  const steps: Array<{ key: WizardStep; label: string }> = [
    { key: 'upload', label: 'Upload' },
    { key: 'source', label: 'Source' },
    { key: 'mapping', label: 'Mapping' },
    { key: 'preview', label: 'Preview' },
    { key: 'execute', label: 'Execute' },
  ];

  $effect(() => {
    void loadReferenceData();
  });

  async function loadReferenceData() {
    try {
      const [routers, packages] = await Promise.all([
        api.mikrotik.routers.list().catch(() => []),
        api.ispPackages.packages.list({ page: 1, per_page: 500, q: '' }).catch(() => ({
          data: [],
        })),
        loadBatchHistory(),
      ]);
      routerOptions = (routers as any[]).map((router) => ({
        label: router.name,
        value: router.id,
      }));
      packageOptions = ((packages as any).data || []).map((pkg: any) => ({
        label: pkg.name,
        value: pkg.id,
      }));
    } catch {
      routerOptions = [];
      packageOptions = [];
    }
  }

  async function loadBatchHistory() {
    historyLoading = true;
    try {
      const response = await api.mixradiusImport.list({ page: 1, per_page: 8 });
      batchHistory = buildMixradiusBatchHistory(response.data ?? []);
    } catch {
      batchHistory = [];
    } finally {
      historyLoading = false;
    }
  }

  async function pickBackup(file?: File) {
    if (file) {
      selectedFile = file;
      selectedPath = '';
      selectedName = file.name;
      selectedSize = file.size;
      error = '';
      return;
    }

    if (!isDesktopApp) {
      error = 'Browser upload membutuhkan file picker web.';
      return;
    }

    const { open } = await import('@tauri-apps/plugin-dialog');
    const { stat } = await import('@tauri-apps/plugin-fs');
    const selected = await open({
      multiple: false,
      filters: [{ name: 'MixRadius SQL', extensions: ['sql', 'gz'] }],
    });
    if (!selected || typeof selected !== 'string') return;
    selectedFile = null;
    selectedPath = selected;
    selectedName = selected.split(/[\\/]/).pop() || selected;
    try {
      const info = await stat(selected);
      selectedSize = Number(info.size || 0);
    } catch {
      selectedSize = 0;
    }
    error = '';
  }

  async function stageUpload() {
    if (!selectedPath && !selectedFile) return;
    uploading = true;
    error = '';
    executionResult = null;
    preview = null;
    initialPreview = null;
    try {
      batch = await api.mixradiusImport.upload({
        file_name: selectedName || 'mixradius.sql.gz',
        file_size_bytes: selectedSize || 1,
        local_path: selectedPath || undefined,
        file: selectedFile,
        content_type: selectedFile?.type || undefined,
      });
      initialPreview = await api.mixradiusImport.preview({
        batch_id: batch.id,
        mapping_overrides: [],
      });
      preview = initialPreview;
      step = 'source';
      await loadBatchHistory();
      toast.success('Backup MixRadius berhasil distaging.');
    } catch (e: any) {
      error = e?.message || String(e) || 'Gagal upload backup MixRadius';
      toast.error(error);
    } finally {
      uploading = false;
    }
  }

  async function buildPreview() {
    if (!batch) return;
    loading = true;
    try {
      preview = await api.mixradiusImport.preview({
        batch_id: batch.id,
        mapping_overrides: mappingOverrides,
        customer_conflict_resolution: customerConflictResolution,
        location_strategy: locationStrategy,
        pppoe_provisioning_target: pppoeProvisioningTarget,
      });
      previewTab = 'all';
      step = 'preview';
    } catch (e: any) {
      toast.error(e?.message || e || 'Gagal membangun preview MixRadius');
    } finally {
      loading = false;
    }
  }

  async function executeImport() {
    if (!batch) return;
    executing = true;
    try {
      executionResult = await api.mixradiusImport.execute({
        batch_id: batch.id,
        execution_mode: executionMode,
        mapping_overrides: mappingOverrides,
        customer_conflict_resolution: customerConflictResolution,
        location_strategy: locationStrategy,
        pppoe_provisioning_target: pppoeProvisioningTarget,
      });
      batch = executionResult.batch;
      preview = executionResult.preview ?? preview;
      executionMode = executionResult.summary.mode;
      await loadBatchHistory();
      toast.success('Import MixRadius selesai diproses.');
    } catch (e: any) {
      const message = e?.message || e || 'Gagal menjalankan import MixRadius';
      if (batch && typeof message === 'string' && message.includes('timed out')) {
        try {
          const refreshed = await api.mixradiusImport.get(batch.id);
          batch = refreshed;
          await loadBatchHistory();
          toast.error(
            'Request execute timeout di client, tetapi backend mungkin masih memproses batch. Cek status terbaru lalu resume bila perlu.'
          );
        } catch {
          toast.error(message);
        }
      } else {
        toast.error(message);
      }
    } finally {
      executing = false;
    }
  }

  async function cancelBatch() {
    if (!batch) {
      goto(`${tenantPrefix}/admin/network/import`);
      return;
    }
    try {
      batch = await api.mixradiusImport.cancel(batch.id);
      await loadBatchHistory();
      toast.success('Batch import dibatalkan.');
      goto(`${tenantPrefix}/admin/network/import`);
    } catch (e: any) {
      toast.error(e?.message || e || 'Gagal membatalkan batch import');
    }
  }

  async function resumeBatch(batchId: string) {
    loading = true;
    error = '';
    try {
      const persisted = await api.mixradiusImport.get(batchId);
      batch = persisted;

      const previewRequest = (persisted.progressJson?.previewRequest ??
        {}) as Record<string, unknown>;
      const executeRequest = (persisted.progressJson?.executeRequest ??
        {}) as Record<string, unknown>;

      mappingOverrides = Array.isArray(previewRequest.mappingOverrides)
        ? (previewRequest.mappingOverrides as MixradiusImportMappingOverrideInput[])
        : [];
      customerConflictResolution =
        (previewRequest.customerConflictResolution as MixradiusImportConflictResolution | null) ??
        null;
      locationStrategy =
        (previewRequest.locationStrategy as MixradiusImportLocationStrategy | null) ?? null;
      pppoeProvisioningTarget =
        (executeRequest.pppoeProvisioningTarget as
          | MixradiusImportPppoeProvisioningTarget
          | undefined) ??
        (previewRequest.pppoeProvisioningTarget as
          | MixradiusImportPppoeProvisioningTarget
          | undefined) ??
        'managed_radius';
      executionMode =
        (executeRequest.executionMode as MixradiusImportExecutionMode | undefined) ??
        persisted.executionMode ??
        'safe_import';

      if (persisted.parseStatus === 'ready') {
        initialPreview = await api.mixradiusImport.preview({
          batch_id: persisted.id,
          mapping_overrides: [],
        });
        preview = await api.mixradiusImport.preview({
          batch_id: persisted.id,
          mapping_overrides: mappingOverrides,
          customer_conflict_resolution: customerConflictResolution,
          location_strategy: locationStrategy,
          pppoe_provisioning_target: pppoeProvisioningTarget,
        });
        previewTab = 'all';
        step = resolveMixradiusResumeStep({
          parseStatus: persisted.parseStatus,
          executionStatus: persisted.executionStatus,
          hasPreviewRequest: Object.keys(previewRequest).length > 0,
          hasExecuteRequest: Object.keys(executeRequest).length > 0,
        });
      } else {
        initialPreview = null;
        preview = null;
        executionResult = null;
        step = 'source';
      }
    } catch (e: any) {
      error = e?.message || String(e) || 'Gagal memuat batch import sebelumnya';
      toast.error(error);
    } finally {
      loading = false;
    }
  }

  async function openBatchReport(batchId: string) {
    reportOpen = true;
    reportLoading = true;
    try {
      reportBatch = await api.mixradiusImport.get(batchId);
    } catch (e: any) {
      reportBatch = null;
      toast.error(e?.message || e || 'Gagal memuat report batch');
    } finally {
      reportLoading = false;
    }
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title="Import MixRadius"
    subtitle="Wizard migrasi backup MixRadius ke ISP Management dengan preview, mapping, dan execute mode."
  >
    {#snippet actions()}
      <button class="btn ghost" type="button" onclick={() => goto(`${tenantPrefix}/admin/network/import`)}>
        <Icon name="arrow-left" size={16} />
        Kembali ke Import
      </button>
    {/snippet}
  </NetworkPageHeader>

  <div class="wizard-shell">
    <div class="wizard-rail">
      <div class="history-block">
        <div class="history-head">
          <strong>Batch terbaru</strong>
          <button class="btn ghost small" type="button" onclick={() => void loadBatchHistory()}>
            Refresh
          </button>
        </div>
        {#if historyLoading}
          <div class="history-empty">Memuat history import...</div>
        {:else if batchHistory.length === 0}
          <div class="history-empty">Belum ada batch MixRadius tersimpan.</div>
        {:else}
          <div class="history-list">
            {#each batchHistory as item}
              <div class:active={batch?.id === item.id} class="history-item">
                <div class="history-item-head">
                  <strong>{item.title}</strong>
                  <span class={`status ${item.status.tone}`}>{item.status.label}</span>
                </div>
                <div class="history-meta">
                  <small>{item.customerCount.toLocaleString()} customer PPP</small>
                  <small>{new Date(item.updatedAt).toLocaleString('id-ID')}</small>
                </div>
                <div class="history-actions">
                  <button class="btn ghost small" type="button" onclick={() => void resumeBatch(item.id)}>
                    Resume
                  </button>
                  <button class="btn ghost small" type="button" onclick={() => void openBatchReport(item.id)}>
                    Report
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#each steps as item, index}
        <div class:active={item.key === step} class="rail-item">
          <span>{index + 1}</span>
          <div>
            <strong>{item.label}</strong>
            {#if item.key === step}
              <small>Current step</small>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <div class="wizard-main">
      {#if step === 'upload'}
        <MixRadiusUploadStep
          {isDesktopApp}
          filePath={selectedPath}
          fileName={selectedName}
          fileSizeBytes={selectedSize}
          {uploading}
          {error}
          onPick={pickBackup}
          onUpload={stageUpload}
        />
      {:else if step === 'source'}
        <MixRadiusSourceSummaryStep
          {batch}
          onBack={() => (step = 'upload')}
          onNext={() => (step = 'mapping')}
        />
      {:else if step === 'mapping'}
        <MixRadiusMappingStep
          rows={initialPreview?.rows ?? []}
          {routerOptions}
          {packageOptions}
          bind:mappingOverrides
          bind:customerConflictResolution
          bind:locationStrategy
          bind:pppoeProvisioningTarget
          loading={loading}
          onBack={() => (step = 'source')}
          onPreview={buildPreview}
        />
      {:else if step === 'preview'}
        <MixRadiusPreviewStep
          {preview}
          bind:activeTab={previewTab}
          onBack={() => (step = 'mapping')}
          onNext={() => (step = 'execute')}
        />
      {:else if step === 'execute'}
        <MixRadiusExecutionStep
          rows={preview?.rows ?? []}
          bind:executionMode
          {executing}
          result={executionResult}
          onBack={() => (step = 'preview')}
          onCancel={cancelBatch}
          onExecute={executeImport}
        />
      {/if}
    </div>
  </div>
</div>

<MixRadiusBatchReportDrawer
  open={reportOpen}
  batch={reportBatch}
  loading={reportLoading}
  onClose={() => {
    reportOpen = false;
    reportBatch = null;
  }}
/>

<style>
  .page-content {
    padding: 28px;
    max-width: 1460px;
    margin: 0 auto;
  }

  .wizard-shell {
    display: grid;
    grid-template-columns: minmax(220px, 280px) minmax(0, 1fr);
    gap: 18px;
  }

  .wizard-rail,
  .wizard-main {
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: var(--bg-card, rgba(15, 23, 42, 0.72));
    border-radius: 24px;
  }

  .wizard-rail {
    padding: 16px;
    display: grid;
    gap: 10px;
    align-content: start;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .wizard-main {
    padding: 22px;
  }

  .rail-item {
    display: flex;
    gap: 12px;
    padding: 12px;
    border-radius: 16px;
    color: var(--text-secondary);
  }

  .rail-item span {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.15);
    font-weight: 700;
  }

  .rail-item.active {
    background: rgba(14, 165, 233, 0.16);
    color: var(--text-primary);
  }

  .rail-item.active span {
    background: rgba(14, 165, 233, 0.9);
    color: white;
  }

  .rail-item strong,
  .rail-item small {
    display: block;
  }

  .rail-item small {
    margin-top: 4px;
    color: var(--text-secondary);
  }

  .history-block {
    display: grid;
    gap: 10px;
    padding-bottom: 8px;
    margin-bottom: 4px;
    border-bottom: 1px solid rgba(148, 163, 184, 0.16);
  }

  .history-head,
  .history-item-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    align-items: center;
  }

  .history-list {
    display: grid;
    gap: 8px;
  }

  .history-item,
  .history-empty {
    border: 1px solid rgba(148, 163, 184, 0.18);
    background: rgba(15, 23, 42, 0.3);
    border-radius: 16px;
    padding: 10px 12px;
  }

  .history-item {
    display: grid;
    gap: 10px;
    color: inherit;
  }

  .history-item.active {
    border-color: rgba(56, 189, 248, 0.5);
    background: rgba(14, 165, 233, 0.1);
  }

  .history-item-head {
    display: grid;
    gap: 8px;
    align-items: start;
  }

  .history-item-head strong {
    font-size: 0.98rem;
    line-height: 1.45;
    color: var(--text-primary);
    word-break: break-word;
    overflow-wrap: anywhere;
  }

  .history-item small,
  .history-empty {
    color: var(--text-secondary);
  }

  .history-meta {
    display: grid;
    gap: 4px;
  }

  .history-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 2px;
  }

  .status {
    justify-self: start;
    width: fit-content;
    font-size: 0.75rem;
    padding: 4px 8px;
    border-radius: 999px;
    border: 1px solid currentColor;
  }

  .status.success {
    color: #86efac;
  }

  .status.warning {
    color: #fde68a;
  }

  .status.danger {
    color: #fca5a5;
  }

  .status.muted {
    color: #cbd5e1;
  }

  @media (max-width: 920px) {
    .page-content {
      padding: 20px;
    }

    .wizard-shell {
      grid-template-columns: 1fr;
    }

    .wizard-rail {
      grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    }
  }
</style>

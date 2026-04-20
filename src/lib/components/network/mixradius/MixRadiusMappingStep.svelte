<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import type {
    MixradiusImportConflictResolution,
    MixradiusImportLocationStrategy,
    MixradiusImportMappingOverrideInput,
    MixradiusImportPppoeProvisioningTarget,
    MixradiusImportPreviewRow,
  } from './mixradiusImportTypes';
  import { getMixradiusProvisioningTargetLabel } from './mixradiusImportTypes';

  type Option = { label: string; value: string };

  let {
    rows = [],
    routerOptions = [],
    packageOptions = [],
    mappingOverrides = $bindable<MixradiusImportMappingOverrideInput[]>([]),
    customerConflictResolution = $bindable<MixradiusImportConflictResolution | null>(null),
    locationStrategy = $bindable<MixradiusImportLocationStrategy | null>(null),
    pppoeProvisioningTarget =
      $bindable<MixradiusImportPppoeProvisioningTarget>('managed_radius'),
    loading = false,
    onBack,
    onPreview,
  }: {
    rows?: MixradiusImportPreviewRow[];
    routerOptions?: Option[];
    packageOptions?: Option[];
    mappingOverrides: MixradiusImportMappingOverrideInput[];
    customerConflictResolution: MixradiusImportConflictResolution | null;
    locationStrategy: MixradiusImportLocationStrategy | null;
    pppoeProvisioningTarget: MixradiusImportPppoeProvisioningTarget;
    loading?: boolean;
    onBack: () => void;
    onPreview: () => void | Promise<void>;
  } = $props();

  const nasRows = $derived.by(() => rows.filter((row) => row.sourceKind === 'nas'));
  const planRows = $derived.by(() => rows.filter((row) => row.sourceKind === 'plan'));
  const provisioningTargets: MixradiusImportPppoeProvisioningTarget[] = ['managed_radius', 'router'];

  function selectedTarget(sourceKind: string, sourceRef: string, targetKind: string) {
    return (
      mappingOverrides.find(
        (item) =>
          item.source_kind === sourceKind &&
          item.source_value === sourceRef &&
          item.target_kind === targetKind
      )?.target_value ?? ''
    );
  }

  function setOverride(sourceKind: string, sourceRef: string, targetKind: string, targetValue: string) {
    const kept = mappingOverrides.filter(
      (item) =>
        !(
          item.source_kind === sourceKind &&
          item.source_value === sourceRef &&
          item.target_kind === targetKind
        )
    );
    mappingOverrides = targetValue
      ? [
          ...kept,
          {
            source_kind: sourceKind,
            source_value: sourceRef,
            target_kind: targetKind,
            target_value: targetValue,
          },
        ]
      : kept;
  }
</script>

<section class="mix-step">
  <div class="section-head">
    <div>
      <h2>Mapping & strategi import</h2>
      <p>Pilih target router/package lokal dan strategi saat data customer/lokasi perlu direview.</p>
    </div>
  </div>

  <div class="mapping-grid">
    <article class="panel">
      <h3>NAS ke router</h3>
      {#if nasRows.length === 0}
        <p class="muted">Tidak ada NAS row di preview awal.</p>
      {:else}
        {#each nasRows as row}
          <label class="field">
            <span>{row.displayName || row.sourceRef}</span>
            <select
              class="input"
              value={selectedTarget('nas', row.sourceRef, 'router')}
              onchange={(event) =>
                setOverride('nas', row.sourceRef, 'router', event.currentTarget.value)}
            >
              <option value="">Pilih router...</option>
              {#each routerOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </label>
        {/each}
      {/if}
    </article>

    <article class="panel">
      <h3>Plan ke package</h3>
      {#if planRows.length === 0}
        <p class="muted">Tidak ada plan row di preview awal.</p>
      {:else}
        {#each planRows as row}
          <label class="field">
            <span>{row.displayName || row.sourceRef}</span>
            <select
              class="input"
              value={selectedTarget('plan', row.sourceRef, 'package')}
              onchange={(event) =>
                setOverride('plan', row.sourceRef, 'package', event.currentTarget.value)}
            >
              <option value="">Auto/new package</option>
              {#each packageOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </label>
        {/each}
      {/if}
    </article>
  </div>

  <div class="mapping-grid">
    <article class="panel">
      <h3>Target provisioning PPPoE</h3>
      <p class="muted">
        Tentukan apakah akun hasil import dibuat sebagai secret lokal router atau sebagai akun
        Managed RADIUS.
      </p>
      <div class="target-list">
        {#each provisioningTargets as target}
          {@const meta = getMixradiusProvisioningTargetLabel(target)}
          <label class:active={pppoeProvisioningTarget === target} class="target-card">
            <input type="radio" bind:group={pppoeProvisioningTarget} value={target} />
            <div>
              <strong>{meta.label}</strong>
              <span>{meta.description}</span>
            </div>
          </label>
        {/each}
      </div>
    </article>

    <label class="field panel">
      <span>Customer conflict decision</span>
      <select class="input" bind:value={customerConflictResolution}>
        <option value={null}>Default safe review</option>
        <option value="merge">Merge</option>
        <option value="create_new">Create new</option>
        <option value="skip">Skip</option>
      </select>
    </label>

    <label class="field panel">
      <span>Location strategy</span>
      <select class="input" bind:value={locationStrategy}>
        <option value={null}>Default</option>
        <option value="preserve">Preserve</option>
        <option value="merge">Merge</option>
        <option value="replace">Replace</option>
      </select>
    </label>
  </div>

  <div class="step-actions">
    <button class="btn ghost" type="button" onclick={onBack}>Back</button>
    <button class="btn primary" type="button" onclick={onPreview} disabled={loading}>
      {loading ? 'Building preview...' : 'Build preview'}
      <Icon name="arrow-right" size={16} />
    </button>
  </div>
</section>

<style>
  .mix-step {
    display: grid;
    gap: 16px;
  }

  .section-head,
  .step-actions {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  .mapping-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 14px;
  }

  .target-list {
    display: grid;
    gap: 10px;
  }

  .panel {
    display: grid;
    gap: 12px;
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: var(--bg-card, rgba(15, 23, 42, 0.72));
    border-radius: 18px;
    padding: 16px;
  }

  .field {
    display: grid;
    gap: 7px;
  }

  .field span,
  .muted {
    color: var(--text-secondary);
    font-size: 0.88rem;
  }

  .target-card {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 14px;
    padding: 12px;
  }

  .target-card.active {
    border-color: rgba(56, 189, 248, 0.5);
    background: rgba(14, 165, 233, 0.08);
  }

  .target-card strong,
  .target-card span {
    display: block;
  }

  .target-card span {
    margin-top: 4px;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.45;
  }
</style>

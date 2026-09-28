<script lang="ts">
  import { t } from 'svelte-i18n';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    canCopyManagedRadiusSecret,
    getManagedRadiusDisplayedSecret,
    shouldShowAssignDefaultManagedRadius,
    shouldShowCreateManagedRadiusMapping,
    shouldShowManagedRadiusUpgrade,
  } from '$lib/utils/managedRadiusSetup';
  import type { ManagedRadiusRouterSetup as ManagedRadiusRouterSetupResponse } from '$lib/api/types';

  let {
    showManagedRadiusModal = $bindable(false),
    managedRadiusSetupLoading = false,
    managedRadiusSetup = null,
    canRevealManagedRadiusSecret = false,
    showManagedRadiusSecret = $bindable(false),
    assigningManagedRadiusDefault = false,
    creatingManagedRadiusMapping = false,
    applyingManagedRadius = false,
    copyManagedRadiusSecret,
    copyManagedRadiusScript,
    assignManagedRadiusDefault,
    createManagedRadiusMapping,
    applyManagedRadius,
    showInterfaceTrafficModal = $bindable(false),
    selectedInterface = null,
    ifaceHistoryLoading = false,
    ifaceHistoryLength = 0,
    rxSeries = [],
    txSeries = [],
    selectedInterfaceRxRate = '—',
    selectedInterfaceTxRate = '—',
    formatBps,
    closeManagedRadiusModal,
    closeInterfaceTrafficModal,
  }: {
    showManagedRadiusModal?: boolean;
    managedRadiusSetupLoading?: boolean;
    managedRadiusSetup?: ManagedRadiusRouterSetupResponse | null;
    canRevealManagedRadiusSecret?: boolean;
    showManagedRadiusSecret?: boolean;
    assigningManagedRadiusDefault?: boolean;
    creatingManagedRadiusMapping?: boolean;
    applyingManagedRadius?: boolean;
    copyManagedRadiusSecret: () => void;
    copyManagedRadiusScript: () => void;
    assignManagedRadiusDefault: () => void;
    createManagedRadiusMapping: () => void;
    applyManagedRadius: () => void;
    showInterfaceTrafficModal?: boolean;
    selectedInterface?: string | null;
    ifaceHistoryLoading?: boolean;
    ifaceHistoryLength?: number;
    rxSeries?: number[];
    txSeries?: number[];
    selectedInterfaceRxRate?: string;
    selectedInterfaceTxRate?: string;
    formatBps: (bps?: number | null) => string;
    closeManagedRadiusModal: () => void;
    closeInterfaceTrafficModal: () => void;
  } = $props();
</script>

<Modal
  bind:show={showManagedRadiusModal}
  title={$t('admin.network.routers.managed_radius.title')}
  width="820px"
  onclose={closeManagedRadiusModal}
>
  <div class="managed-radius-modal-head">
    <div>
      <div class="muted">
        {$t('admin.network.routers.managed_radius.subtitle')}
      </div>
    </div>
    <div class="setup-actions">
      {#if shouldShowAssignDefaultManagedRadius(managedRadiusSetup)}
        <button
          class="btn btn-sm"
          type="button"
          onclick={assignManagedRadiusDefault}
          disabled={assigningManagedRadiusDefault}
        >
          <Icon name="shield-check" size={14} />
          {#if assigningManagedRadiusDefault}
            {$t('common.loading')}
          {:else}
            {$t('admin.network.routers.managed_radius.actions.assign_default')}
          {/if}
        </button>
      {/if}

      {#if shouldShowCreateManagedRadiusMapping(managedRadiusSetup)}
        <button
          class="btn btn-sm"
          type="button"
          onclick={createManagedRadiusMapping}
          disabled={creatingManagedRadiusMapping}
        >
          <Icon name="plus-circle" size={14} />
          {#if creatingManagedRadiusMapping}
            {$t('common.loading')}
          {:else}
            {$t('admin.network.routers.managed_radius.actions.create_mapping')}
          {/if}
        </button>
      {/if}

      {#if managedRadiusSetup?.configured && canRevealManagedRadiusSecret && canCopyManagedRadiusSecret(managedRadiusSetup)}
        <button
          class="btn ghost btn-sm"
          type="button"
          onclick={() => (showManagedRadiusSecret = !showManagedRadiusSecret)}
        >
          <Icon name={showManagedRadiusSecret ? 'eye-off' : 'eye'} size={14} />
          {showManagedRadiusSecret
            ? $t('admin.network.routers.managed_radius.actions.hide_secret') || 'Hide secret'
            : $t('admin.network.routers.managed_radius.actions.show_secret') || 'Show secret'}
        </button>

        <button class="btn ghost btn-sm" type="button" onclick={copyManagedRadiusSecret}>
          <Icon name="copy" size={14} />
          {$t('admin.network.routers.managed_radius.actions.copy_secret')}
        </button>
      {/if}

      {#if managedRadiusSetup?.configured && managedRadiusSetup.cli_script}
        <button class="btn btn-sm" type="button" onclick={applyManagedRadius} disabled={applyingManagedRadius}>
          <Icon name="play" size={14} />
          {#if applyingManagedRadius}
            {$t('common.loading')}
          {:else}
            {$t('admin.network.routers.managed_radius.actions.apply')}
          {/if}
        </button>

        <button class="btn ghost btn-sm" type="button" onclick={copyManagedRadiusScript}>
          <Icon name="copy" size={14} />
          {$t('admin.network.routers.managed_radius.actions.copy_cli')}
        </button>
      {/if}
    </div>
  </div>

  {#if managedRadiusSetupLoading && !managedRadiusSetup}
    <div class="muted">{$t('common.loading')}</div>
  {:else if shouldShowManagedRadiusUpgrade(managedRadiusSetup)}
    <div class="setup-upgrade">
      <div class="setup-warning">
        <Icon name="alert-triangle" size={16} />
        <div>
          <div class="strong">
            {$t('admin.network.routers.managed_radius.upgrade.title')}
          </div>
          <div>
            {$t('admin.network.routers.managed_radius.upgrade.body')}
          </div>
        </div>
      </div>

      <a class="btn btn-sm" href={managedRadiusSetup?.upgrade_path || '/admin/subscription'}>
        {$t('admin.network.routers.managed_radius.upgrade.cta')}
      </a>
    </div>
  {:else if managedRadiusSetup?.configured}
    <div class="setup-grid">
      <div class="row">
        <span class="muted">
          {$t('admin.network.routers.managed_radius.labels.server')}
        </span>
        <span class="mono">{managedRadiusSetup.endpoint_name || 'Managed RADIUS'}</span>
      </div>
      <div class="row">
        <span class="muted">
          {$t('admin.network.routers.managed_radius.labels.host')}
        </span>
        <span class="mono">{managedRadiusSetup.radius_host || '—'}</span>
      </div>
      <div class="row">
        <span class="muted">
          {$t('admin.network.routers.managed_radius.labels.ports')}
        </span>
        <span class="mono"
          >auth {managedRadiusSetup.auth_port} / acct {managedRadiusSetup.acct_port}</span
        >
      </div>
      <div class="row">
        <span class="muted">
          {$t('admin.network.routers.managed_radius.labels.nas_source')}
        </span>
        <span class="mono">{managedRadiusSetup.nas_ip_or_cidr || '—'}</span>
      </div>
      <div class="row">
        <span class="muted">
          {$t('admin.network.routers.managed_radius.labels.shared_secret')}
        </span>
        <span class="mono"
          >{getManagedRadiusDisplayedSecret(managedRadiusSetup, showManagedRadiusSecret)}</span
        >
      </div>
    </div>

    {#if managedRadiusSetup.warnings.length}
      <div class="setup-warning">
        <Icon name="alert-triangle" size={16} />
        <div>
          {#each managedRadiusSetup.warnings as warning}
            <div>{warning}</div>
          {/each}
        </div>
      </div>
    {/if}

    {#if managedRadiusSetup.cli_script}
      <pre class="code-block">{managedRadiusSetup.cli_script}</pre>
    {/if}
  {:else if managedRadiusSetup?.tenant_has_active_assignment}
    <div class="setup-upgrade">
      <div class="row">
        <span class="muted">
          {$t('admin.network.routers.managed_radius.labels.server')}
        </span>
        <span class="mono">{managedRadiusSetup.assignment_endpoint_name || 'Managed RADIUS'}</span>
      </div>
      <div class="muted">
        {$t('admin.network.routers.managed_radius.assignment_only')}
      </div>
      <div class="muted">
        {$t('admin.network.routers.managed_radius.assignment_only_hint')}
      </div>
    </div>
  {:else if managedRadiusSetup?.default_server_available}
    <div class="setup-upgrade">
      <div class="muted">
        {$t('admin.network.routers.managed_radius.default_ready')}
      </div>
    </div>
  {:else}
    <div class="muted">
      {$t('admin.network.routers.managed_radius.empty')}
    </div>
  {/if}
</Modal>

<Modal
  bind:show={showInterfaceTrafficModal}
  title={$t('network.router.interface_traffic')}
  width="980px"
  onclose={closeInterfaceTrafficModal}
>
  {#if selectedInterface}
    <div class="traffic-modal-head">
      <span class="muted">{$t('network.router.interface')}</span>
      <span class="mono">{selectedInterface}</span>
    </div>

    {#if ifaceHistoryLoading}
      <div class="muted">{$t('common.loading')}</div>
    {:else if ifaceHistoryLength === 0}
      <div class="muted">{$t('network.router.no_history_yet')}</div>
    {:else}
      <div class="traffic-grid">
        <div class="traffic-card">
          <div class="traffic-top">
            <span class="muted">RX</span>
            <span class="mono">{selectedInterfaceRxRate}</span>
          </div>
          <div class="spark small">
            {#if rxSeries.length === 0}
              <div class="muted">{$t('network.router.no_rx_samples')}</div>
            {:else}
              {@const max = Math.max(...rxSeries, 1)}
              {#each rxSeries as v}
                <div
                  class="bar rx"
                  style={`height:${Math.round((v / max) * 100)}%;`}
                  title={formatBps(v)}
                ></div>
              {/each}
            {/if}
          </div>
        </div>

        <div class="traffic-card">
          <div class="traffic-top">
            <span class="muted">TX</span>
            <span class="mono">{selectedInterfaceTxRate}</span>
          </div>
          <div class="spark small">
            {#if txSeries.length === 0}
              <div class="muted">{$t('network.router.no_tx_samples')}</div>
            {:else}
              {@const max = Math.max(...txSeries, 1)}
              {#each txSeries as v}
                <div
                  class="bar tx"
                  style={`height:${Math.round((v / max) * 100)}%;`}
                  title={formatBps(v)}
                ></div>
              {/each}
            {/if}
          </div>
        </div>
      </div>
    {/if}
  {/if}
</Modal>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 800;
    cursor: pointer;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn-sm {
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 0.85rem;
  }

  .setup-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .managed-radius-modal-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .muted {
    color: var(--text-secondary);
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    color: var(--text-primary);
  }

  .setup-grid {
    display: grid;
    gap: 10px;
  }

  .setup-upgrade {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }

  .setup-warning {
    margin-top: 12px;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 14px;
    border: 1px solid rgba(245, 158, 11, 0.28);
    background: rgba(245, 158, 11, 0.1);
    color: rgba(245, 158, 11, 0.95);
    font-weight: 700;
  }

  .strong {
    font-weight: 700;
    margin-bottom: 4px;
  }

  .code-block {
    margin: 12px 0 0;
    padding: 14px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    background: #0f172a;
    color: #e2e8f0;
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.9rem;
    line-height: 1.5;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
  }

  .traffic-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .traffic-modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-surface), transparent 12%);
  }

  .traffic-card {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
    border-radius: var(--radius-lg);
    padding: 12px;
  }

  .traffic-top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 10px;
    font-weight: 900;
  }

  .spark {
    height: 140px;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 2px;
    align-items: end;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    padding: 10px;
    background: color-mix(in srgb, var(--bg-surface), transparent 8%);
    overflow: hidden;
  }

  .spark.small {
    height: 120px;
  }

  .bar {
    width: 100%;
    background: rgba(99, 102, 241, 0.72);
    border-radius: 6px 6px 2px 2px;
    opacity: 0.95;
  }

  .bar.rx {
    background: rgba(34, 197, 94, 0.72);
  }

  .bar.tx {
    background: rgba(99, 102, 241, 0.72);
  }

  @media (max-width: 900px) {
    .traffic-grid {
      grid-template-columns: 1fr;
    }
  }
</style>

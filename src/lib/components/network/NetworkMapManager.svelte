<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    computeLinkHealth,
    isSystemManagedNode,
    nodeTypeLabel,
    systemManagedNodeSourceLabel,
    type NMLink,
    type NMNode,
    type NMZone,
  } from '$lib/components/network/networkMapUtils';

  type SelectedTab = 'nodes' | 'links' | 'zones' | 'bindings';
  type BindingForm = {
    zone_id: string;
    node_id: string;
    is_primary: boolean;
    weight: string;
  };

  let {
    manageMode,
    title,
    subtitle,
    selectedTab,
    nodeRows,
    linkRows,
    zoneRows,
    zoneBindings,
    selectedZoneId,
    loadingManager,
    savingBinding,
    deletingId,
    bindingForm,
    onClose,
    onSelectTab,
    onOpenCreateNode,
    onOpenCreateLink,
    onOpenCreateZone,
    onStartConnectNode,
    onOpenEditNode,
    onOpenEditLink,
    onOpenEditZone,
    onOpenDeleteConfirm,
    onSelectedZoneChange,
    onBindingNodeChange,
    onBindingWeightChange,
    onBindingPrimaryChange,
    onCreateBinding,
  }: {
    manageMode: boolean;
    title: string;
    subtitle: string;
    selectedTab: SelectedTab;
    nodeRows: NMNode[];
    linkRows: NMLink[];
    zoneRows: NMZone[];
    zoneBindings: any[];
    selectedZoneId: string;
    loadingManager: boolean;
    savingBinding: boolean;
    deletingId: string | null;
    bindingForm: BindingForm;
    onClose: () => void;
    onSelectTab: (tab: SelectedTab) => void;
    onOpenCreateNode: () => void;
    onOpenCreateLink: () => void;
    onOpenCreateZone: () => void;
    onStartConnectNode: (id: string) => void;
    onOpenEditNode: (row: NMNode) => void;
    onOpenEditLink: (row: NMLink) => void;
    onOpenEditZone: (row: NMZone) => void;
    onOpenDeleteConfirm: (
      type: 'node' | 'link' | 'zone' | 'binding',
      id: string,
      name?: string,
    ) => void;
    onSelectedZoneChange: (value: string) => void;
    onBindingNodeChange: (value: string) => void;
    onBindingWeightChange: (value: string) => void;
    onBindingPrimaryChange: (checked: boolean) => void;
    onCreateBinding: () => void;
  } = $props();
</script>

{#if manageMode}
  <div class="manager-wrap">
    <div class="manager-header">
      <div class="manager-heading">
        <div class="manager-kicker">{$t('network.map.manage_mode')}</div>
        <div class="manager-title">{title}</div>
        <div class="manager-subtitle">{subtitle}</div>
      </div>

      <div class="manager-header-main">
        <div class="manager-tabs">
          <button class:active={selectedTab === 'nodes'} onclick={() => onSelectTab('nodes')}
            >{$t('network.map.nodes')}</button
          >
          <button class:active={selectedTab === 'links'} onclick={() => onSelectTab('links')}
            >{$t('network.map.links')}</button
          >
          <button class:active={selectedTab === 'zones'} onclick={() => onSelectTab('zones')}
            >{$t('network.map.zones')}</button
          >
          <button class:active={selectedTab === 'bindings'} onclick={() => onSelectTab('bindings')}>
            {$t('network.map.zone_bindings')}
          </button>
        </div>

        <div class="manager-actions">
          {#if selectedTab === 'nodes'}
            <button class="btn" type="button" onclick={onOpenCreateNode}>
              <Icon name="plus" size={14} />
              {$t('network.map.add_node')}
            </button>
          {:else if selectedTab === 'links'}
            <button class="btn" type="button" onclick={onOpenCreateLink}>
              <Icon name="plus" size={14} />
              {$t('network.map.add_link')}
            </button>
          {:else if selectedTab === 'zones'}
            <button class="btn" type="button" onclick={onOpenCreateZone}>
              <Icon name="plus" size={14} />
              {$t('network.map.add_zone')}
            </button>
          {/if}
          <button class="btn ghost" type="button" onclick={onClose}>
            <Icon name="x-circle" size={14} />
            {$t('common.close')}
          </button>
        </div>
      </div>
    </div>

    {#if selectedTab === 'nodes'}
      <div class="table-wrap">
        <div class="table-top"><strong>{nodeRows.length}</strong> {$t('network.map.nodes_in_viewport')}</div>
        <table class="table">
          <thead>
            <tr>
              <th>{$t('common.name')}</th>
              <th>{$t('common.type')}</th>
              <th>{$t('network.map.source')}</th>
              <th>{$t('common.status')}</th>
              <th>{$t('network.map.coordinates')}</th>
              <th class="right">{$t('common.actions')}</th>
            </tr>
          </thead>
          <tbody>
            {#if nodeRows.length === 0}
              <tr><td colspan="6" class="empty">{$t('network.map.no_nodes')}</td></tr>
            {:else}
              {#each nodeRows as row}
                <tr>
                  <td>{row.name}</td>
                  <td>{nodeTypeLabel(row.node_type)}</td>
                  <td>{systemManagedNodeSourceLabel(row) || $t('network.map.manual') || 'Manual'}</td>
                  <td>{row.status}</td>
                  <td>{row.lat.toFixed(6)}, {row.lng.toFixed(6)}</td>
                  <td class="right">
                    <button class="btn ghost btn-xs" onclick={() => onStartConnectNode(row.id)}
                      >{$t('network.map.connect')}</button
                    >
                    <button
                      class="btn ghost btn-xs"
                      onclick={() => onOpenEditNode(row)}
                      disabled={isSystemManagedNode(row)}
                    >
                      {$t('common.edit')}
                    </button>
                    <button
                      class="btn ghost btn-xs danger"
                      onclick={() => onOpenDeleteConfirm('node', row.id, row.name)}
                      disabled={deletingId === row.id || isSystemManagedNode(row)}
                    >
                      {$t('common.delete')}
                    </button>
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    {/if}

    {#if selectedTab === 'links'}
      <div class="table-wrap">
        <div class="table-top"><strong>{linkRows.length}</strong> {$t('network.map.links_in_viewport')}</div>
        <table class="table">
          <thead>
            <tr>
              <th>{$t('common.name')}</th>
              <th>{$t('common.type')}</th>
              <th>{$t('common.status')}</th>
              <th>{$t('network.map.health')}</th>
              <th>{$t('network.map.endpoints')}</th>
              <th class="right">{$t('common.actions')}</th>
            </tr>
          </thead>
          <tbody>
            {#if linkRows.length === 0}
              <tr><td colspan="6" class="empty">{$t('network.map.no_links')}</td></tr>
            {:else}
              {#each linkRows as row}
                {@const health = computeLinkHealth(row)}
                <tr>
                  <td>{row.name}</td>
                  <td>{row.link_type}</td>
                  <td>{row.status}</td>
                  <td>
                    <span class={`health-pill ${health.tone}`}>{health.score}</span>
                  </td>
                  <td>{row.from_node_id || '-'} -> {row.to_node_id || '-'}</td>
                  <td class="right">
                    <button class="btn ghost btn-xs" onclick={() => onOpenEditLink(row)}
                      >{$t('common.edit')}</button
                    >
                    <button
                      class="btn ghost btn-xs danger"
                      onclick={() => onOpenDeleteConfirm('link', row.id, row.name)}
                      disabled={deletingId === row.id}>{$t('common.delete')}</button
                    >
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    {/if}

    {#if selectedTab === 'zones'}
      <div class="table-wrap">
        <div class="table-top"><strong>{zoneRows.length}</strong> {$t('network.map.zones_in_viewport')}</div>
        <table class="table">
          <thead>
            <tr>
              <th>{$t('common.name')}</th>
              <th>{$t('common.type')}</th>
              <th>{$t('common.status')}</th>
              <th class="right">{$t('common.actions')}</th>
            </tr>
          </thead>
          <tbody>
            {#if zoneRows.length === 0}
              <tr><td colspan="4" class="empty">{$t('network.map.no_zones')}</td></tr>
            {:else}
              {#each zoneRows as row}
                <tr>
                  <td>{row.name}</td>
                  <td>{row.zone_type}</td>
                  <td>{row.status}</td>
                  <td class="right">
                    <button class="btn ghost btn-xs" onclick={() => onOpenEditZone(row)}
                      >{$t('common.edit')}</button
                    >
                    <button
                      class="btn ghost btn-xs danger"
                      onclick={() => onOpenDeleteConfirm('zone', row.id, row.name)}
                      disabled={deletingId === row.id}>{$t('common.delete')}</button
                    >
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    {/if}

    {#if selectedTab === 'bindings'}
      <div class="bindings-wrap">
        <div class="binding-form">
          <div class="control">
            <label for="zone-id">{$t('network.map.zone')}</label>
            <select
              id="zone-id"
              class="input"
              value={selectedZoneId}
              onchange={(e) => onSelectedZoneChange((e.currentTarget as HTMLSelectElement).value)}
            >
              <option value="">{$t('network.map.select_zone')}</option>
              {#each zoneRows as z}
                <option value={z.id}>{z.name}</option>
              {/each}
            </select>
          </div>
          <div class="control">
            <label for="node-id">{$t('network.map.node')}</label>
            <select
              id="node-id"
              class="input"
              value={bindingForm.node_id}
              disabled={!selectedZoneId}
              onchange={(e) => onBindingNodeChange((e.currentTarget as HTMLSelectElement).value)}
            >
              <option value="">{$t('network.map.select_node')}</option>
              {#each nodeRows as n}
                <option value={n.id}>{n.name}</option>
              {/each}
            </select>
          </div>
          <div class="control">
            <label for="binding-weight">{$t('network.map.weight')}</label>
            <input
              id="binding-weight"
              class="input"
              type="number"
              min="1"
              value={bindingForm.weight}
              oninput={(e) => onBindingWeightChange((e.currentTarget as HTMLInputElement).value)}
            />
          </div>
          <label class="toggle">
            <input
              type="checkbox"
              checked={bindingForm.is_primary}
              onchange={(e) =>
                onBindingPrimaryChange((e.currentTarget as HTMLInputElement).checked)}
            />
            <span>{$t('network.map.primary')}</span>
          </label>
          <button
            class="btn"
            type="button"
            onclick={onCreateBinding}
            disabled={!selectedZoneId || savingBinding}
          >
            <Icon name="plus" size={14} />
            {$t('network.map.add_binding')}
          </button>
        </div>

        <div class="table-wrap">
          <div class="table-top"><strong>{zoneBindings.length}</strong> {$t('network.map.bindings')}</div>
          <table class="table">
            <thead>
              <tr>
                <th>{$t('network.map.zone')}</th>
                <th>{$t('network.map.node')}</th>
                <th>{$t('network.map.primary')}</th>
                <th>{$t('network.map.weight')}</th>
                <th class="right">{$t('common.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {#if loadingManager}
                <tr><td colspan="5" class="empty">{$t('common.loading')}</td></tr>
              {:else if zoneBindings.length === 0}
                <tr><td colspan="5" class="empty">{$t('network.map.no_bindings')}</td></tr>
              {:else}
                {#each zoneBindings as row}
                  <tr>
                    <td>{zoneRows.find((z) => z.id === row.zone_id)?.name || row.zone_id}</td>
                    <td>{nodeRows.find((n) => n.id === row.node_id)?.name || row.node_id}</td>
                    <td>{row.is_primary ? ($t('common.yes') || 'Yes') : ($t('common.no') || 'No')}</td>
                    <td>{row.weight}</td>
                    <td class="right">
                      <button
                        class="btn ghost btn-xs danger"
                        onclick={() => onOpenDeleteConfirm('binding', row.id)}
                        disabled={deletingId === row.id}>{$t('common.delete')}</button
                      >
                    </td>
                  </tr>
                {/each}
              {/if}
            </tbody>
          </table>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .manager-wrap {
    margin-top: 14px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    overflow: hidden;
  }

  .manager-header {
    padding: 14px 14px 12px;
    display: grid;
    gap: 12px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-surface);
  }

  .manager-heading {
    display: grid;
    gap: 4px;
  }

  .manager-kicker {
    font-size: 0.68rem;
    font-weight: 900;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--color-primary) 72%, white 28%);
  }

  .manager-title {
    color: var(--text-primary);
    font-size: 1rem;
    font-weight: 900;
  }

  .manager-subtitle {
    color: var(--text-secondary);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .manager-header-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .manager-tabs {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .manager-tabs button {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 94%, #0a1020 6%);
    color: var(--text-secondary);
    border-radius: 10px;
    padding: 8px 10px;
    font-weight: 700;
    cursor: pointer;
  }

  .manager-tabs button.active {
    color: var(--text-primary);
    border-color: color-mix(in srgb, var(--color-primary) 50%, var(--border-color));
    background: var(--bg-surface);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.08),
      0 4px 16px rgba(56, 96, 255, 0.2);
  }

  .manager-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .table-wrap {
    overflow-x: auto;
  }

  .table-top {
    padding: 10px 12px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .table {
    width: 100%;
    border-collapse: collapse;
    min-width: 720px;
  }

  .table th,
  .table td {
    text-align: left;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-color);
    font-size: 0.88rem;
  }

  .table th {
    color: var(--text-secondary);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 0.72rem;
  }

  .table .right {
    text-align: right;
    white-space: nowrap;
  }

  .table .empty {
    text-align: center;
    color: var(--text-secondary);
    padding: 18px 12px;
  }

  .bindings-wrap {
    display: grid;
    gap: 10px;
    padding: 10px;
  }

  .binding-form {
    display: grid;
    gap: 10px;
    grid-template-columns: repeat(4, minmax(0, 1fr)) auto auto;
    align-items: end;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 10px;
  }

  .control {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .control label {
    font-size: 0.78rem;
    color: #cbd5e1;
    font-weight: 700;
  }

  .input {
    width: 100%;
    border: 1px solid #334155;
    border-radius: 10px;
    background: #111827;
    color: #e5e7eb;
    padding: 10px 12px;
    font-size: 0.9rem;
    outline: none;
  }

  .input:focus {
    border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-color));
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 22%, transparent);
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.86rem;
    color: var(--text-secondary);
  }

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
    text-decoration: none;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn-xs {
    padding: 6px 10px;
    font-size: 0.78rem;
    border-radius: 9px;
  }

  .btn.danger {
    color: #fca5a5;
    border-color: color-mix(in srgb, #ef4444 55%, var(--border-color));
  }

  @media (max-width: 960px) {
    .manager-header-main {
      flex-direction: column;
      align-items: stretch;
    }

    .binding-form {
      grid-template-columns: 1fr;
    }
  }
</style>

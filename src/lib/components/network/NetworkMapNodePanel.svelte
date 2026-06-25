<script lang="ts">
  import Select2 from '$lib/components/ui/Select2.svelte';

  export let show = false;
  export let editingNodeId: string | null = null;
  export let nodePickMode = false;
  export let savingNode = false;
  export let nodeForm: {
    name: string;
    node_type: string;
    status: string;
    lat: string;
    lng: string;
  };
  export let nodeTypeOptions: Array<{ label: string; value: string }> = [];
  export let onClose: () => void;
  export let onSubmit: () => void;
</script>

{#if show}
  <div class="node-create-panel">
    <div class="panel-head">
      <div class="panel-title">{editingNodeId ? ($t('network.map.edit_node') || 'Edit Node') : ($t('network.map.add_node') || 'Add Node')}</div>
      {!editingNodeId
        ? (nodePickMode
            ? ($t('network.map.pick_mode_active') || 'Pick mode active: click map to set node position, drag marker for precision.')
            : ($t('network.map.pick_mode_paused') || 'Pick mode paused.'))
        : ($t('network.map.edit_node_hint') || 'Edit node details and save changes.')}
    </div>
    <div class="form-grid two-col">
      <label class="field">
        <span>{$t('common.name') || 'Name'}</span>
        <input class="input" bind:value={nodeForm.name} />
      </label>
      <label class="field">
        <span>{$t('common.type') || 'Type'}</span>
        <Select2
          bind:value={nodeForm.node_type}
          options={nodeTypeOptions}
          width="100%"
          placeholder={$t('network.map.select_node_type') || 'Select node type'}
          searchPlaceholder={$t('common.search') || 'Search type...'}
          noResultsText={$t('common.not_found') || 'No type found'}
        />
      </label>
      <label class="field">
        <span>{$t('common.status') || 'Status'}</span>
        <select class="input" bind:value={nodeForm.status}>
          <option value="active">active</option>
          <option value="inactive">inactive</option>
          <option value="maintenance">maintenance</option>
        </select>
      </label>
      {#if !editingNodeId}
        <label class="field">
          <span>{$t('network.map.latitude') || 'Latitude'}</span>
          <input class="input" type="number" step="0.000001" bind:value={nodeForm.lat} />
        </label>
        <label class="field">
          <span>{$t('network.map.longitude') || 'Longitude'}</span>
          <input class="input" type="number" step="0.000001" bind:value={nodeForm.lng} />
        </label>
      {:else}
        <div class="field field-full node-edit-location-hint">
          <span>{$t('common.location') || 'Location'}</span>
          <div class="hint-card">
            {$t('network.map.drag_marker_hint') || 'Marker is draggable on map. Drag marker to update node position.'}
            <div class="hint-coord">{nodeForm.lat}, {nodeForm.lng}</div>
          </div>
        </div>
      {/if}
    </div>
    <div class="node-panel-actions">
      <button class="btn ghost" type="button" onclick={onClose} disabled={savingNode}>{$t('common.cancel') || 'Cancel'}</button>
      <button class="btn" type="button" onclick={onSubmit} disabled={savingNode}>
        {savingNode ? ($t('common.saving') || 'Saving...') : editingNodeId ? ($t('network.map.update_node') || 'Update Node') : ($t('network.map.save_node') || 'Save Node')}
      </button>
    </div>
  </div>
{/if}

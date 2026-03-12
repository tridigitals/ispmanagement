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
      <div class="panel-title">{editingNodeId ? 'Edit Node' : 'Add Node'}</div>
      {!editingNodeId
        ? (nodePickMode
            ? 'Pick mode active: click map to set node position, drag marker for precision.'
            : 'Pick mode paused.')
        : 'Edit node details and save changes.'}
    </div>
    <div class="form-grid two-col">
      <label class="field">
        <span>Name</span>
        <input class="input" bind:value={nodeForm.name} />
      </label>
      <label class="field">
        <span>Type</span>
        <Select2
          bind:value={nodeForm.node_type}
          options={nodeTypeOptions}
          width="100%"
          placeholder="Select node type"
          searchPlaceholder="Search type..."
          noResultsText="No type found"
        />
      </label>
      <label class="field">
        <span>Status</span>
        <select class="input" bind:value={nodeForm.status}>
          <option value="active">active</option>
          <option value="inactive">inactive</option>
          <option value="maintenance">maintenance</option>
        </select>
      </label>
      {#if !editingNodeId}
        <label class="field">
          <span>Latitude</span>
          <input class="input" type="number" step="0.000001" bind:value={nodeForm.lat} />
        </label>
        <label class="field">
          <span>Longitude</span>
          <input class="input" type="number" step="0.000001" bind:value={nodeForm.lng} />
        </label>
      {:else}
        <div class="field field-full node-edit-location-hint">
          <span>Location</span>
          <div class="hint-card">
            Marker is draggable on map. Drag marker to update node position.
            <div class="hint-coord">{nodeForm.lat}, {nodeForm.lng}</div>
          </div>
        </div>
      {/if}
    </div>
    <div class="node-panel-actions">
      <button class="btn ghost" type="button" onclick={onClose} disabled={savingNode}>Cancel</button>
      <button class="btn" type="button" onclick={onSubmit} disabled={savingNode}>
        {savingNode ? 'Saving...' : editingNodeId ? 'Update Node' : 'Save Node'}
      </button>
    </div>
  </div>
{/if}

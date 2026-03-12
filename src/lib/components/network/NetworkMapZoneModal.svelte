<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';

  export let show = false;
  export let editingZoneId: string | null = null;
  export let savingZone = false;
  export let zoneForm: {
    name: string;
    zone_type: string;
    status: string;
    priority: string;
    geometryText: string;
  };
  export let onClose: () => void;
  export let onSubmit: () => void;
</script>

<Modal
  {show}
  title={editingZoneId ? 'Edit Zone' : 'Add Zone'}
  width="860px"
  onclose={() => !savingZone && onClose()}
>
  <div class="form-grid two-col">
    <label class="field">
      <span>Name</span>
      <input class="input" bind:value={zoneForm.name} />
    </label>
    <label class="field">
      <span>Type</span>
      <input class="input" bind:value={zoneForm.zone_type} />
    </label>
    <label class="field">
      <span>Status</span>
      <select class="input" bind:value={zoneForm.status}>
        <option value="active">active</option>
        <option value="inactive">inactive</option>
      </select>
    </label>
    <label class="field">
      <span>Priority</span>
      <input class="input" type="number" min="1" bind:value={zoneForm.priority} />
    </label>
    <label class="field field-full">
      <span>Geometry (GeoJSON Polygon/MultiPolygon)</span>
      <textarea class="input textarea" rows="9" bind:value={zoneForm.geometryText}></textarea>
    </label>
  </div>
  {#snippet footer()}
    <button class="btn ghost" type="button" onclick={onClose} disabled={savingZone}>Cancel</button>
    <button class="btn" type="button" onclick={onSubmit} disabled={savingZone}>
      {savingZone ? 'Saving...' : 'Save'}
    </button>
  {/snippet}
</Modal>

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
  title={editingZoneId ? ($t('network.map.edit_zone') || 'Edit Zone') : ($t('network.map.add_zone') || 'Add Zone')}
  width="860px"
  onclose={() => !savingZone && onClose()}
>
  <div class="form-grid two-col">
    <label class="field">
      <span>{$t('common.name')}</span>
      <input class="input" bind:value={zoneForm.name} />
    </label>
    <label class="field">
      <span>{$t('common.type')}</span>
      <input class="input" bind:value={zoneForm.zone_type} />
    </label>
    <label class="field">
      <span>{$t('common.status')}</span>
      <select class="input" bind:value={zoneForm.status}>
        <option value="active">active</option>
        <option value="inactive">inactive</option>
      </select>
    </label>
    <label class="field">
      <span>{$t('network.map.priority')}</span>
      <input class="input" type="number" min="1" bind:value={zoneForm.priority} />
    </label>
    <label class="field field-full">
      <span>{$t('network.map.geometry')}</span>
      <textarea class="input textarea" rows="9" bind:value={zoneForm.geometryText}></textarea>
    </label>
  </div>
  {#snippet footer()}
    <button class="btn ghost" type="button" onclick={onClose} disabled={savingZone}>{$t('common.cancel')}</button>
    <button class="btn" type="button" onclick={onSubmit} disabled={savingZone}>
      {savingZone ? ($t('common.saving') || 'Saving...') : ($t('common.save') || 'Save')}
    </button>
  {/snippet}
</Modal>

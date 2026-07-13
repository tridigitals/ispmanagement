<script lang="ts">
  import { onMount } from 'svelte';
  import type { Component } from 'svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { api, type CustomerLocation } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { loadLocationFormModal } from './dashboardLocationsPageModules';

  let loading = $state(true);
  let locations = $state<CustomerLocation[]>([]);
  let error = $state('');

  let showLocationModal = $state(false);
  let editingLocation: CustomerLocation | null = $state(null);
  let savingLocation = $state(false);
  let LocationFormModalComponent = $state<Component<any> | null>(null);
  let showDeleteDialog = $state(false);
  let deletingLocation = $state(false);
  let deleteLocationId = $state<string | null>(null);

  let fLabel = $state('');
  let fLine1 = $state('');
  let fLine2 = $state('');
  let fCity = $state('');
  let fState = $state('');
  let fPostal = $state('');
  let fCountry = $state('ID');
  let fNotes = $state('');
  let fLatitude = $state('');
  let fLongitude = $state('');

  const hasLinkedCustomer = $derived($can('read_own', 'customers'));
  const totalLocations = $derived(locations.length);
  const mappedLocations = $derived(
    locations.filter((loc) => loc.latitude != null && loc.longitude != null).length,
  );
  const notedLocations = $derived(locations.filter((loc) => (loc.notes || '').trim().length > 0).length);

  onMount(async () => {
    await load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      locations = hasLinkedCustomer ? await api.customers.portal.myLocations() : [];
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to load locations');
      toast.error(get(t)('dashboard.locations.toasts.load_failed') || 'Failed to load locations');
    } finally {
      loading = false;
    }
  }

  function formatAddress(loc: CustomerLocation) {
    return [
      loc.address_line1,
      loc.address_line2,
      [loc.city, loc.state, loc.postal_code].filter(Boolean).join(', '),
      loc.country,
    ]
      .filter((part) => Boolean(part && String(part).trim()))
      .join(' • ');
  }

  function resetForm() {
    editingLocation = null;
    fLabel = '';
    fLine1 = '';
    fLine2 = '';
    fCity = '';
    fState = '';
    fPostal = '';
    fCountry = 'ID';
    fNotes = '';
    fLatitude = '';
    fLongitude = '';
  }

  async function ensureLocationFormModalComponent() {
    if (LocationFormModalComponent) return;

    const modules = await loadLocationFormModal();
    LocationFormModalComponent = modules.LocationFormModalComponent;
  }

  async function openCreateLocation() {
    resetForm();
    await ensureLocationFormModalComponent();
    showLocationModal = true;
  }

  async function openEditLocation(loc: CustomerLocation) {
    editingLocation = loc;
    fLabel = loc.label || '';
    fLine1 = loc.address_line1 || '';
    fLine2 = loc.address_line2 || '';
    fCity = loc.city || '';
    fState = loc.state || '';
    fPostal = loc.postal_code || '';
    fCountry = loc.country || 'ID';
    fNotes = loc.notes || '';
    fLatitude = loc.latitude != null ? String(loc.latitude) : '';
    fLongitude = loc.longitude != null ? String(loc.longitude) : '';
    await ensureLocationFormModalComponent();
    showLocationModal = true;
  }

  function parseCoordOrNull(v: string) {
    const raw = v.trim();
    if (!raw) return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : NaN;
  }

  function validateLocationForm() {
    const label = fLabel.trim();
    if (!label) {
      toast.error('Label lokasi wajib diisi');
      return null;
    }

    const latitude = parseCoordOrNull(fLatitude);
    const longitude = parseCoordOrNull(fLongitude);
    if (latitude == null || longitude == null) {
      toast.error('Lokasi wajib dipilih di map');
      return null;
    }
    if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) {
      toast.error('Koordinat lokasi tidak valid');
      return null;
    }
    if (latitude < -90 || latitude > 90) {
      toast.error('Latitude harus di antara -90 hingga 90');
      return null;
    }
    if (longitude < -180 || longitude > 180) {
      toast.error('Longitude harus di antara -180 hingga 180');
      return null;
    }

    return {
      label,
      address_line1: fLine1.trim() || null,
      address_line2: fLine2.trim() || null,
      city: fCity.trim() || null,
      state: fState.trim() || null,
      postal_code: fPostal.trim() || null,
      country: fCountry.trim().toUpperCase() || null,
      notes: fNotes.trim() || null,
      latitude,
      longitude,
    };
  }

  async function saveLocation() {
    const payload = validateLocationForm();
    if (!payload) return;

    savingLocation = true;
    try {
      if (editingLocation) {
        await api.customers.portal.updateMyLocation(editingLocation.id, payload);
      } else {
        await api.customers.portal.createMyLocation(payload);
      }
      showLocationModal = false;
      resetForm();
      await load();
      toast.success($t('common.saved') || 'Saved');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Failed to save location'));
    } finally {
      savingLocation = false;
    }
  }

  function askDeleteLocation(locationId: string) {
    deleteLocationId = locationId;
    showDeleteDialog = true;
  }

  async function doDeleteLocation() {
    if (!deleteLocationId) return;
    deletingLocation = true;
    try {
      await api.customers.portal.deleteMyLocation(deleteLocationId);
      showDeleteDialog = false;
      deleteLocationId = null;
      await load();
      toast.success($t('common.deleted') || 'Deleted');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Failed to delete location'));
    } finally {
      deletingLocation = false;
    }
  }

</script>

<div class="page-container fade-in">
  <div class="hero-card locations-hero">
    <div class="hero-left">
      <div class="hero-badge">
        <Icon name="map-pin" size={20} />
      </div>
      <div>
        <div class="kicker">
          <span class="dot"></span>
          {$t('dashboard.locations.kicker')}
        </div>
        <h1 class="hero-title">{$t('dashboard.locations.title')}</h1>
        <p class="hero-sub">{$t('dashboard.locations.subtitle')}</p>
      </div>
    </div>
    <div class="hero-actions">
      <button class="btn btn-secondary btn-sm" type="button" onclick={load} disabled={loading}>
        <Icon name="refresh-cw" size={14} />
        <span>{$t('common.refresh')}</span>
      </button>
      <button class="btn btn-primary btn-sm" type="button" onclick={openCreateLocation} disabled={loading || !hasLinkedCustomer}>
        <Icon name="plus" size={14} />
        <span>Tambah Lokasi</span>
      </button>
    </div>
  </div>

  <div class="bento-grid stats-bento">
    <div class="bento-card">
      <div class="bento-icon">
        <Icon name="map-pin" size={18} />
      </div>
      <div class="bento-value">{totalLocations}</div>
      <div class="bento-label">{$t('dashboard.locations.summary.total')}</div>
    </div>
    <div class="bento-card">
      <div class="bento-icon">
        <Icon name="navigation" size={18} />
      </div>
      <div class="bento-value">{mappedLocations}</div>
      <div class="bento-label">{$t('dashboard.locations.summary.pinned')}</div>
    </div>
    <div class="bento-card">
      <div class="bento-icon">
        <Icon name="file-text" size={18} />
      </div>
      <div class="bento-value">{notedLocations}</div>
      <div class="bento-label">{$t('dashboard.locations.summary.has_notes')}</div>
    </div>
  </div>

  {#if !hasLinkedCustomer}
    <div class="error-banner">
      <Icon name="alert-triangle" size={18} />
      <span>{$t('dashboard.locations.no_customer_hint')}</span>
    </div>
  {/if}

  {#if error}
    <div class="error-banner">
      <Icon name="alert-triangle" size={18} />
      <span>{error}</span>
    </div>
  {/if}

  {#if loading}
    <div class="loading-card card">
      <div class="spinner"></div>
      <p>{$t('common.loading')}</p>
    </div>
  {:else if locations.length === 0}
    <div class="empty glass-card">
      <Icon name="map-pin" size={32} />
      <div class="empty-text">
        <div class="title">{$t('dashboard.locations.empty')}</div>
        <div class="sub">{$t('dashboard.locations.empty_hint')}</div>
      </div>
    </div>
  {:else}
    <div class="grid">
      {#each locations as loc (loc.id)}
        <div class="location glass-card">
          <div class="top">
            <div class="badge">
              <Icon name="map-pin" size={16} />
              <span>{$t('dashboard.locations.title')}</span>
            </div>
            <div class="row-actions">
              <button class="btn-icon" title={$t('common.edit')} onclick={() => openEditLocation(loc)}>
                <Icon name="edit" size={14} />
              </button>
              <button class="btn-icon danger" title={$t('common.delete')} onclick={() => askDeleteLocation(loc.id)}>
                <Icon name="trash-2" size={14} />
              </button>
            </div>
          </div>
          <div class="name">{loc.label || 'Location'}</div>
          <div class="addr">{formatAddress(loc) || 'Alamat belum diisi'}</div>
          <div class="coords">
            {#if loc.latitude != null && loc.longitude != null}
              <span class="coord-chip">{Number(loc.latitude).toFixed(6)}, {Number(loc.longitude).toFixed(6)}</span>
            {:else}
              <span class="coord-chip missing">{$t('dashboard.locations.form.no_map_pin')}</span>
            {/if}
          </div>
          {#if loc.notes}
            <div class="notes">{loc.notes}</div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if LocationFormModalComponent}
  <LocationFormModalComponent
    bind:show={showLocationModal}
    {editingLocation}
    {savingLocation}
    bind:fLabel
    bind:fLine1
    bind:fLine2
    bind:fCity
    bind:fState
    bind:fPostal
    bind:fCountry
    bind:fNotes
    bind:fLatitude
    bind:fLongitude
    onSave={saveLocation}
  />
{/if}

<ConfirmDialog
  show={showDeleteDialog}
  title={$t('common.delete')}
  message="Lokasi ini akan dihapus dari akun customer. Lanjutkan?"
  confirmText={$t('common.delete')}
  cancelText={$t('common.cancel')}
  loading={deletingLocation}
  onconfirm={doDeleteLocation}
  oncancel={() => (showDeleteDialog = false)}
/>

<style>
  .page-container {
    padding: clamp(1rem, 2.2vw, 2rem);
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .locations-hero {
    margin-bottom: 0;
  }

  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.35rem;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--color-primary);
    box-shadow: 0 0 0 6px var(--color-primary-subtle);
  }

  .stats-bento {
    margin-bottom: 0;
  }

  .error-banner {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--color-danger) 30%, var(--border-color));
    background: color-mix(in srgb, var(--color-danger) 8%, transparent);
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
  }

  .location {
    padding: 1.15rem;
    position: relative;
  }

  .top {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.9rem;
  }

  .row-actions {
    display: flex;
    gap: 0.45rem;
  }

  .btn-icon {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 10px;
    padding: 0.35rem 0.45rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.15s;
  }

  .btn-icon:hover {
    border-color: rgba(255,255,255,0.2);
  }

  .btn-icon.danger {
    border-color: color-mix(in srgb, var(--color-danger) 35%, var(--border-color));
    color: var(--color-danger);
  }

  .btn-icon:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .badge,
  .coord-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 650;
  }

  .coord-chip.missing {
    border-color: color-mix(in srgb, var(--color-warning) 40%, var(--border-color));
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  }

  .name {
    font-size: 1.1rem;
    font-weight: 750;
    margin-bottom: 0.35rem;
    color: var(--text-primary);
  }

  .addr {
    color: var(--text-secondary);
    line-height: 1.4;
    font-size: 0.95rem;
  }

  .coords {
    margin-top: 0.85rem;
  }

  .notes {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
    color: var(--text-secondary);
    font-size: 0.9rem;
    white-space: pre-wrap;
  }

  .empty {
    padding: 1.5rem;
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
  }

  .empty-text .title {
    font-weight: 750;
    margin-bottom: 0.25rem;
    color: var(--text-primary);
  }

  .empty-text .sub {
    color: var(--text-secondary);
  }

  .loading-card {
    padding: 1.25rem;
    display: grid;
    place-items: center;
    gap: 0.5rem;
  }

  .spinner {
    width: 26px;
    height: 26px;
    border-radius: 999px;
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary);
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 980px) {
    .page-content {
      padding: 0.95rem;
    }

    .summary-grid {
      grid-template-columns: 1fr;
    }

    .page-header {
      flex-direction: column;
      align-items: stretch;
    }

    .header-actions {
      width: 100%;
      justify-content: stretch;
    }

    .header-actions > button {
      flex: 1 1 auto;
      justify-content: center;
    }
  }
</style>

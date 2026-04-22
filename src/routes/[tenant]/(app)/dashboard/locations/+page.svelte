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

<div class="page-content fade-in">
  <div class="page-header">
    <div>
      <div class="kicker">
        <span class="dot"></span>
        {$t('dashboard.locations.kicker') || 'Customer portal'}
      </div>
      <h1>{$t('dashboard.locations.title') || 'My Locations'}</h1>
      <p class="subtitle">
        Kelola lokasi layanan Anda di sini. Saat membuat atau mengubah lokasi, titik map wajib dipilih.
      </p>
    </div>
    <div class="header-actions">
      <button class="btn-primary" onclick={openCreateLocation} disabled={loading || !hasLinkedCustomer}>
        <Icon name="plus" size={16} />
        Tambah Lokasi
      </button>
      <button class="btn-secondary" onclick={load} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  <div class="summary-grid">
    <div class="summary card">
      <div class="summary-label">Total lokasi</div>
      <div class="summary-value">{totalLocations}</div>
    </div>
    <div class="summary card">
      <div class="summary-label">Sudah pin map</div>
      <div class="summary-value">{mappedLocations}</div>
    </div>
    <div class="summary card">
      <div class="summary-label">Ada catatan</div>
      <div class="summary-value">{notedLocations}</div>
    </div>
  </div>

  {#if !hasLinkedCustomer}
    <div class="error-banner">
      <Icon name="alert-triangle" size={18} />
      <span>Akun ini belum terhubung ke customer, jadi lokasi layanan belum bisa dikelola.</span>
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
      <p>{$t('common.loading') || 'Loading...'}</p>
    </div>
  {:else if locations.length === 0}
    <div class="empty card">
      <Icon name="map-pin" size={28} />
      <div class="empty-text">
        <div class="title">Belum ada lokasi layanan.</div>
        <div class="sub">Tambahkan lokasi baru lalu pilih titik map agar bisa dipakai untuk order dan coverage check.</div>
      </div>
    </div>
  {:else}
    <div class="grid">
      {#each locations as loc (loc.id)}
        <div class="location card">
          <div class="top">
            <div class="badge">
              <Icon name="map-pin" size={16} />
              <span>Service Location</span>
            </div>
            <div class="row-actions">
              <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => openEditLocation(loc)}>
                <Icon name="edit" size={14} />
              </button>
              <button class="btn-icon danger" title={$t('common.delete') || 'Delete'} onclick={() => askDeleteLocation(loc.id)}>
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
              <span class="coord-chip missing">Belum ada titik map</span>
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
  title={$t('common.delete') || 'Delete'}
  message="Lokasi ini akan dihapus dari akun customer. Lanjutkan?"
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deletingLocation}
  onconfirm={doDeleteLocation}
  oncancel={() => (showDeleteDialog = false)}
/>

<style>
  .page-content {
    padding: 1.1rem 1.35rem 1.4rem;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.25rem;
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
    background: rgba(99, 102, 241, 0.9);
    box-shadow: 0 0 0 6px rgba(99, 102, 241, 0.12);
  }

  .subtitle {
    color: var(--text-secondary);
    margin-top: 0.35rem;
    max-width: 720px;
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.8rem;
    margin-bottom: 0.95rem;
  }

  .summary {
    padding: 0.95rem 1rem;
  }

  .summary-label {
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 650;
  }

  .summary-value {
    margin-top: 0.3rem;
    font-size: 1.5rem;
    font-weight: 800;
    color: var(--text-primary);
  }

  .btn-primary,
  .btn-secondary {
    border-radius: 12px;
    padding: 0.55rem 0.85rem;
    border: 1px solid var(--border-color);
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    cursor: pointer;
    font-weight: 700;
  }

  .btn-primary {
    background: rgba(99, 102, 241, 0.95);
    border-color: rgba(99, 102, 241, 0.55);
    color: white;
  }

  .btn-secondary {
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .btn-primary:disabled,
  .btn-secondary:disabled,
  .btn-icon:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error-banner {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    border: 1px solid rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.08);
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
    overflow: hidden;
  }

  .location::before {
    content: '';
    position: absolute;
    inset: -1px;
    background:
      radial-gradient(800px 240px at 0% 0%, rgba(99, 102, 241, 0.18), transparent 55%),
      radial-gradient(900px 260px at 100% 0%, rgba(34, 197, 94, 0.12), transparent 58%);
    pointer-events: none;
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
  }

  .btn-icon.danger {
    border-color: rgba(239, 68, 68, 0.35);
    color: rgb(239, 68, 68);
  }

  .badge,
  .coord-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    border-radius: 999px;
    border: 1px solid rgba(148, 163, 184, 0.35);
    background: rgba(148, 163, 184, 0.08);
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 650;
  }

  .coord-chip.missing {
    border-color: rgba(245, 158, 11, 0.4);
    background: rgba(245, 158, 11, 0.12);
  }

  .name,
  .addr,
  .coords,
  .notes {
    position: relative;
  }

  .name {
    font-size: 1.1rem;
    font-weight: 750;
    margin-bottom: 0.35rem;
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
    padding: 1.2rem;
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
  }

  .empty-text .title {
    font-weight: 750;
    margin-bottom: 0.25rem;
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
    border: 3px solid rgba(148, 163, 184, 0.3);
    border-top-color: rgba(99, 102, 241, 0.9);
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

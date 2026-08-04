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

<div class="page fade-in">
  <div class="page-head">
    <div class="page-head-text">
      <h1>{$t('dashboard.locations.title') || 'Lokasi'}</h1>
      <p class="page-sub">
        {#if loading}
          Memuat lokasi...
        {:else}
          {$t('dashboard.locations.subtitle') || 'Alamat instalasi layanan Anda'}
        {/if}
      </p>
    </div>
    <div class="head-actions">
      <button class="btn btn-ghost" type="button" onclick={load} disabled={loading}>
        <Icon name="refresh-cw" size={14} />
        <span>{$t('common.refresh') || 'Refresh'}</span>
      </button>
      <button
        class="btn btn-primary"
        type="button"
        onclick={openCreateLocation}
        disabled={loading || !hasLinkedCustomer}
      >
        <Icon name="plus" size={14} />
        <span>Tambah lokasi</span>
      </button>
    </div>
  </div>

  {#if !loading}
    <div class="kpis">
      <div class="kpi">
        <div class="kpi-label">{$t('dashboard.locations.summary.total') || 'Total'}</div>
        <div class="kpi-val">{totalLocations}</div>
        <div class="kpi-sub">lokasi tersimpan</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">{$t('dashboard.locations.summary.pinned') || 'Pinned'}</div>
        <div class="kpi-val {mappedLocations > 0 ? 'ok' : ''}">{mappedLocations}</div>
        <div class="kpi-sub">punya pin map</div>
      </div>
      <div class="kpi">
        <div class="kpi-label">{$t('dashboard.locations.summary.has_notes') || 'Catatan'}</div>
        <div class="kpi-val">{notedLocations}</div>
        <div class="kpi-sub">ada notes</div>
      </div>
    </div>
  {/if}

  {#if !hasLinkedCustomer}
    <div class="banner warn">
      <Icon name="alert-triangle" size={18} />
      <span>{$t('dashboard.locations.no_customer_hint') || 'Akun belum terhubung ke customer'}</span>
    </div>
  {/if}

  {#if error}
    <div class="banner bad">
      <Icon name="alert-triangle" size={18} />
      <span>{error}</span>
      <button class="btn btn-ghost btn-sm" type="button" onclick={load}>Coba lagi</button>
    </div>
  {/if}

  {#if loading}
    <div class="panel">
      <div class="state">
        <div class="spinner"></div>
        <p>{$t('common.loading') || 'Loading...'}</p>
      </div>
    </div>
  {:else if locations.length === 0}
    <div class="panel">
      <div class="state">
        <Icon name="map-pin" size={40} />
        <h3>{$t('dashboard.locations.empty') || 'Belum ada lokasi'}</h3>
        <p>{$t('dashboard.locations.empty_hint') || 'Tambah alamat instalasi untuk layanan baru'}</p>
        <button
          class="btn btn-primary"
          type="button"
          onclick={openCreateLocation}
          disabled={!hasLinkedCustomer}
        >
          <Icon name="plus" size={14} />
          Tambah lokasi
        </button>
      </div>
    </div>
  {:else}
    <div class="grid">
      {#each locations as loc (loc.id)}
        <div class="card">
          <div class="card-top">
            <span class="pill">
              <Icon name="map-pin" size={12} />
              {loc.label || 'Lokasi'}
            </span>
            <div class="row-actions">
              <button
                class="icon-btn"
                type="button"
                title={$t('common.edit') || 'Edit'}
                aria-label={$t('common.edit') || 'Edit'}
                onclick={() => openEditLocation(loc)}
              >
                <Icon name="edit" size={14} />
              </button>
              <button
                class="icon-btn danger"
                type="button"
                title={$t('common.delete') || 'Delete'}
                aria-label={$t('common.delete') || 'Delete'}
                onclick={() => askDeleteLocation(loc.id)}
              >
                <Icon name="trash-2" size={14} />
              </button>
            </div>
          </div>
          <div class="name">{loc.label || 'Location'}</div>
          <div class="addr">{formatAddress(loc) || 'Alamat belum diisi'}</div>
          <div class="coords">
            {#if loc.latitude != null && loc.longitude != null}
              <span class="chip mono"
                >{Number(loc.latitude).toFixed(6)}, {Number(loc.longitude).toFixed(6)}</span
              >
            {:else}
              <span class="chip missing"
                >{$t('dashboard.locations.form.no_map_pin') || 'Belum ada pin'}</span
              >
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
  .page {
    padding: clamp(1rem, 2.2vw, 1.75rem);
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .page-head h1 {
    font-size: clamp(1.25rem, 2.2vw, 1.45rem);
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
    color: var(--text-primary);
  }
  .page-sub {
    color: var(--text-secondary);
    font-size: 0.88rem;
    margin: 0.25rem 0 0;
  }
  .head-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .kpis {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.7rem;
  }
  .kpi {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    padding: 0.9rem 1rem;
  }
  .kpi-label {
    font-size: 0.7rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    margin-bottom: 0.35rem;
  }
  .kpi-val {
    font-size: 1.25rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }
  .kpi-val.ok {
    color: var(--color-success);
  }
  .kpi-sub {
    font-size: 0.74rem;
    color: var(--text-secondary);
    margin-top: 0.2rem;
  }

  .banner {
    display: flex;
    gap: 0.55rem;
    align-items: center;
    flex-wrap: wrap;
    padding: 0.75rem 0.9rem;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
    font-size: 0.88rem;
  }
  .banner.warn {
    border-color: color-mix(in srgb, var(--color-warning) 35%, transparent);
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
  }
  .banner.bad {
    border-color: color-mix(in srgb, var(--color-danger) 35%, transparent);
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
  }

  .panel {
    background: var(--bg-surface);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-lg, 12px);
    overflow: hidden;
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    min-height: 240px;
    gap: 0.5rem;
    padding: 2rem 1.25rem;
    color: var(--text-secondary);
  }
  .state h3 {
    margin: 0.4rem 0 0;
    color: var(--text-primary);
    font-size: 1.05rem;
  }
  .state p {
    margin: 0 0 0.75rem;
    font-size: 0.88rem;
    max-width: 320px;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.08);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 0.85rem;
  }
  .card {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 1rem 1.1rem;
  }
  .card-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .row-actions {
    display: flex;
    gap: 0.35rem;
  }
  .name {
    font-size: 1.05rem;
    font-weight: 750;
    color: var(--text-primary);
    margin-bottom: 0.3rem;
  }
  .addr {
    color: var(--text-secondary);
    line-height: 1.45;
    font-size: 0.9rem;
  }
  .coords {
    margin-top: 0.75rem;
  }
  .notes {
    margin-top: 0.7rem;
    padding-top: 0.7rem;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    font-size: 0.88rem;
    white-space: pre-wrap;
  }

  .pill,
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.55rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 650;
  }
  .chip.missing {
    border-color: color-mix(in srgb, var(--color-warning) 35%, transparent);
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    color: var(--color-warning);
  }
  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.78rem;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 8px;
  }
  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
  }
  .icon-btn.danger {
    color: var(--color-danger);
    border-color: color-mix(in srgb, var(--color-danger) 30%, transparent);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem 0.95rem;
    border-radius: 8px;
    font-weight: 650;
    font-size: 0.88rem;
    cursor: pointer;
    border: none;
    min-height: 40px;
  }
  .btn-sm {
    padding: 0.35rem 0.7rem;
    font-size: 0.82rem;
    min-height: 34px;
  }
  .btn-primary {
    background: var(--color-primary);
    color: #fff;
  }
  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .btn-ghost:hover:not(:disabled) {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  @media (max-width: 900px) {
    .kpis {
      grid-template-columns: 1fr 1fr;
    }
  }
  @media (max-width: 560px) {
    .page-head {
      align-items: stretch;
    }
    .head-actions {
      width: 100%;
    }
    .head-actions .btn {
      flex: 1;
      min-height: 44px;
    }
    .kpis {
      grid-template-columns: 1fr;
      gap: 0.55rem;
    }
    .kpi {
      padding: 0.75rem 0.85rem;
    }
  }
</style>

<script lang="ts">
  /*
    Lokasi portal v2 — gelombang 25b.
    Versi lama: (app)/dashboard/locations/+page.svelte (654 baris).
    Perilaku identik: KPI + grid kartu + modal form (lazy, map picker) + hapus.
    Pola DS: PortalShell + PageHeader + StatTile + Card + Button + ConfirmDialog.
  */
  import { onMount } from 'svelte';
  import type { Component } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { api, type CustomerLocation } from '$lib/api/client';
  import { loadLocationFormModal } from '../../../../(app)/dashboard/locations/dashboardLocationsPageModules';
  import PortalShell from '$lib/components/ds/PortalShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

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
  const mappedLocations = $derived(locations.filter((loc) => loc.latitude != null && loc.longitude != null).length);
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
      error = String(e?.message || e || 'Gagal memuat lokasi');
      toast.error('Gagal memuat lokasi');
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
      toast.success('Disimpan');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Gagal menyimpan lokasi'));
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
      toast.success('Dihapus');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Gagal menghapus lokasi'));
    } finally {
      deletingLocation = false;
    }
  }
</script>

<PortalShell title="Lokasi">
  <PageHeader
    title="Lokasi"
    desc={loading ? 'Memuat lokasi…' : 'Alamat instalasi layanan Anda.'}
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={load}>Segarkan</Button>
      <Button icon="plus" onclick={openCreateLocation} disabled={loading || !hasLinkedCustomer}>
        Tambah lokasi
      </Button>
    {/snippet}
  </PageHeader>

  {#if !loading}
    <div class="mb-5 grid grid-cols-3 gap-3">
      <StatTile label="Total" value={String(totalLocations)} hint="lokasi tersimpan" />
      <StatTile
        label="Berpeta"
        value={String(mappedLocations)}
        hint="punya pin map"
        tone={mappedLocations > 0 ? 'positive' : 'neutral'}
      />
      <StatTile label="Bercatatan" value={String(notedLocations)} hint="ada notes" />
    </div>
  {/if}

  {#if !hasLinkedCustomer}
    <div class="banner-warn">Akun belum terhubung ke customer.</div>
  {/if}

  {#if error}
    <div class="banner-bad">
      <span>{error}</span>
      <Button variant="ghost" size="sm" onclick={load}>Coba lagi</Button>
    </div>
  {/if}

  {#if loading}
    <Card title="Memuat…"><p class="text-sm text-ink-500">Mengambil lokasi…</p></Card>
  {:else if locations.length === 0}
    <Card title="Belum ada lokasi">
      <p class="mb-4 text-sm text-ink-500">Tambah alamat instalasi untuk layanan baru.</p>
      <Button icon="plus" onclick={openCreateLocation} disabled={!hasLinkedCustomer}>Tambah lokasi</Button>
    </Card>
  {:else}
    <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      {#each locations as loc (loc.id)}
        <Card title={loc.label || 'Lokasi'}>
          {#snippet aside()}
            <div class="flex gap-1">
              <Button variant="ghost" size="sm" icon="wrench" onclick={() => openEditLocation(loc)}>Edit</Button>
              <Button variant="ghost" size="sm" icon="close" onclick={() => askDeleteLocation(loc.id)}>
                Hapus
              </Button>
            </div>
          {/snippet}
          <p class="text-sm text-ink-600">{formatAddress(loc) || 'Alamat belum diisi'}</p>
          <div class="mt-3 flex flex-wrap items-center gap-2">
            {#if loc.latitude != null && loc.longitude != null}
              <span class="chip">{Number(loc.latitude).toFixed(6)}, {Number(loc.longitude).toFixed(6)}</span>
            {:else}
              <span class="chip chip-missing">Belum ada pin</span>
            {/if}
          </div>
          {#if loc.notes}
            <p class="mt-3 text-xs text-ink-500">{loc.notes}</p>
          {/if}
        </Card>
      {/each}
    </div>
  {/if}
</PortalShell>

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
  title="Hapus lokasi"
  message="Lokasi ini akan dihapus dari akun customer. Lanjutkan?"
  confirmText="Hapus"
  cancelText="Batal"
  type="danger"
  loading={deletingLocation}
  onconfirm={doDeleteLocation}
  oncancel={() => (showDeleteDialog = false)}
/>

<style>
  .banner-warn {
    border-radius: 0.75rem;
    border: 1px solid #fde68a;
    background: #fffbeb;
    color: #92400e;
    padding: 0.7rem 1rem;
    margin-bottom: 1rem;
    font-size: 0.875rem;
  }
  .banner-bad {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    border-radius: 0.75rem;
    border: 1px solid #fecaca;
    background: #fef2f2;
    color: #991b1b;
    padding: 0.7rem 1rem;
    margin-bottom: 1rem;
    font-size: 0.875rem;
  }
  .chip {
    border-radius: 9999px;
    border: 1px solid var(--ink-200, #e7e5e4);
    background: #fff;
    padding: 0.15rem 0.6rem;
    font-size: 0.7rem;
    font-family: ui-monospace, monospace;
    color: var(--ink-500, #78716c);
  }
  .chip-missing {
    border-style: dashed;
    color: var(--ink-400, #a8a29e);
  }
</style>

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

  import { t } from 'svelte-i18n';
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
      error = String(e?.message || e || $t('dashboard.locations.v2.t_load_fail'));
      toast.error($t('dashboard.locations.v2.t_load_fail'));
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
      toast.error($t('dashboard.locations.v2.t_label_req'));
      return null;
    }
    const latitude = parseCoordOrNull(fLatitude);
    const longitude = parseCoordOrNull(fLongitude);
    if (latitude == null || longitude == null) {
      toast.error($t('dashboard.locations.v2.t_map_req'));
      return null;
    }
    if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) {
      toast.error($t('dashboard.locations.v2.t_coord'));
      return null;
    }
    if (latitude < -90 || latitude > 90) {
      toast.error($t('dashboard.locations.v2.t_lat'));
      return null;
    }
    if (longitude < -180 || longitude > 180) {
      toast.error($t('dashboard.locations.v2.t_lng'));
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
      toast.success($t('dashboard.locations.v2.t_saved'));
    } catch (e: any) {
      toast.error(String(e?.message || e || $t('dashboard.locations.v2.t_save_fail')));
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
      toast.success($t('dashboard.locations.v2.t_deleted'));
    } catch (e: any) {
      toast.error(String(e?.message || e || $t('dashboard.locations.v2.t_del_fail')));
    } finally {
      deletingLocation = false;
    }
  }
</script>

<PortalShell title={ $t('dashboard.locations.location') }>
  <PageHeader
    title={ $t('dashboard.locations.location') }
    desc={loading ? $t('dashboard.locations.v2.loading') : $t('dashboard.locations.v2.desc')}
  >
    {#snippet actions()}
      <Button variant="ghost" icon="refresh" disabled={loading} onclick={load}>{ $t('common.refresh') }</Button>
      <Button icon="plus" onclick={openCreateLocation} disabled={loading || !hasLinkedCustomer}>
        { $t('dashboard.locations.v2.add') }
      </Button>
    {/snippet}
  </PageHeader>

  {#if !loading}
    <div class="mb-5 grid grid-cols-3 gap-3">
      <StatTile label={ $t('dashboard.locations.v2.total') } value={String(totalLocations)} hint={ $t('dashboard.locations.v2.h_saved') } />
      <StatTile
        label={ $t('dashboard.locations.v2.mapped') }
        value={String(mappedLocations)}
        hint={ $t('dashboard.locations.v2.h_pin') }
        tone={mappedLocations > 0 ? 'positive' : 'neutral'}
      />
      <StatTile label={ $t('dashboard.locations.v2.noted') } value={String(notedLocations)} hint={ $t('dashboard.locations.v2.h_notes') } />
    </div>
  {/if}

  {#if !hasLinkedCustomer}
    <div class="banner-warn">{ $t('dashboard.locations.v2.no_link') }</div>
  {/if}

  {#if error}
    <div class="banner-bad">
      <span>{error}</span>
      <Button variant="ghost" size="sm" onclick={load}>{ $t('common.retry') }</Button>
    </div>
  {/if}

  {#if loading}
    <Card title={ $t('common.loading') }><p class="text-sm text-ink-500">{ $t('dashboard.locations.v2.fetching') }</p></Card>
  {:else if locations.length === 0}
    <Card title={ $t('dashboard.locations.v2.empty') }>
      <p class="mb-4 text-sm text-ink-500">{ $t('dashboard.locations.v2.empty_hint') }</p>
      <Button icon="plus" onclick={openCreateLocation} disabled={!hasLinkedCustomer}>{ $t('dashboard.locations.v2.add') }</Button>
    </Card>
  {:else}
    <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      {#each locations as loc (loc.id)}
        <Card title={loc.label || $t('dashboard.locations.location')}>
          {#snippet aside()}
            <div class="flex gap-1">
              <Button variant="ghost" size="sm" icon="wrench" onclick={() => openEditLocation(loc)}>{ $t('common.edit') }</Button>
              <Button variant="ghost" size="sm" icon="close" onclick={() => askDeleteLocation(loc.id)}>
                { $t('common.delete') }
              </Button>
            </div>
          {/snippet}
          <p class="text-sm text-ink-600">{formatAddress(loc) || $t('dashboard.locations.v2.no_addr')}</p>
          <div class="mt-3 flex flex-wrap items-center gap-2">
            {#if loc.latitude != null && loc.longitude != null}
              <span class="chip">{Number(loc.latitude).toFixed(6)}, {Number(loc.longitude).toFixed(6)}</span>
            {:else}
              <span class="chip chip-missing">{ $t('dashboard.locations.v2.no_pin') }</span>
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
  title={ $t('dashboard.locations.v2.del_title') }
  message={ $t('dashboard.locations.v2.del_body') }
  confirmText={ $t('common.delete') }
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

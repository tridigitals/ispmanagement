<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { toast } from 'svelte-sonner';
  import { can } from '$lib/stores/auth';
  import { api, type CustomerLocation, type UserAddress } from '$lib/api/client';

  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  let loading = $state(true);
  let locations = $state<CustomerLocation[]>([]);
  let userAddresses = $state<UserAddress[]>([]);
  let error = $state('');
  const totalLocations = $derived((locations?.length || 0) + (userAddresses?.length || 0));

  let showAddressModal = $state(false);
  let editingAddressId = $state<string | null>(null);
  let savingAddress = $state(false);
  let showDeleteAddress = $state(false);
  let deletingAddress = $state(false);
  let deleteAddressId = $state<string | null>(null);

  let fLabel = $state('');
  let fRecipient = $state('');
  let fPhone = $state('');
  let fLine1 = $state('');
  let fLine2 = $state('');
  let fCity = $state('');
  let fState = $state('');
  let fPostal = $state('');
  let fCountry = $state('ID');
  let fDefaultShipping = $state(false);
  let fDefaultBilling = $state(false);

  type MergedLocation = {
    id: string;
    title: string;
    address: string;
    notes: string;
    source: string;
    isProfile: boolean;
    profileId?: string;
  };

  onMount(async () => {
    await load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      locations = [];
      userAddresses = [];

      if ($can('read_own', 'customers')) {
        locations = await api.customers.portal.myLocations();
      }
      // Fallback/source for normal user accounts: profile multi-address
      userAddresses = await api.users.listMyAddresses();
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to load locations');
      toast.error(get(t)('dashboard.locations.toasts.load_failed') || 'Failed to load locations');
    } finally {
      loading = false;
    }
  }

  function formatAddress(loc: CustomerLocation) {
    const line1 = loc.address_line1 || '';
    const parts = [loc.city, loc.state, loc.postal_code, loc.country].filter(Boolean).join(', ');
    return [line1, parts].filter(Boolean).join(' • ');
  }

  function formatUserAddress(addr: UserAddress) {
    const line1 = addr.line1 || '';
    const parts = [addr.city, addr.state, addr.postal_code, addr.country_code].filter(Boolean).join(', ');
    return [line1, parts].filter(Boolean).join(' • ');
  }

  const mergedLocations = $derived.by<MergedLocation[]>(() => {
    const fromCustomer = (locations || []).map((loc) => ({
      id: `customer-${loc.id}`,
      title: loc.label || 'Location',
      address: formatAddress(loc),
      notes: loc.notes || '',
      source: $t('dashboard.locations.source.customer') || 'Customer',
      isProfile: false,
    }));

    const fromProfile = (userAddresses || []).map((addr) => ({
      id: `profile-${addr.id}`,
      title: addr.label || addr.recipient_name || 'Address',
      address: formatUserAddress(addr),
      notes: '',
      source: $t('dashboard.locations.source.profile') || 'Profile',
      isProfile: true,
      profileId: addr.id,
    }));

    return [...fromCustomer, ...fromProfile];
  });

  function resetAddressForm() {
    editingAddressId = null;
    fLabel = '';
    fRecipient = '';
    fPhone = '';
    fLine1 = '';
    fLine2 = '';
    fCity = '';
    fState = '';
    fPostal = '';
    fCountry = 'ID';
    fDefaultShipping = false;
    fDefaultBilling = false;
  }

  function openCreateAddress() {
    resetAddressForm();
    showAddressModal = true;
  }

  function openEditAddress(addr: UserAddress) {
    editingAddressId = addr.id;
    fLabel = addr.label || '';
    fRecipient = addr.recipient_name || '';
    fPhone = addr.phone || '';
    fLine1 = addr.line1 || '';
    fLine2 = addr.line2 || '';
    fCity = addr.city || '';
    fState = addr.state || '';
    fPostal = addr.postal_code || '';
    fCountry = addr.country_code || 'ID';
    fDefaultShipping = !!addr.is_default_shipping;
    fDefaultBilling = !!addr.is_default_billing;
    showAddressModal = true;
  }

  async function saveAddress() {
    if (!fLine1.trim()) return;
    savingAddress = true;
    try {
      const payload = {
        label: fLabel.trim() || undefined,
        recipientName: fRecipient.trim() || undefined,
        phone: fPhone.trim() || undefined,
        line1: fLine1.trim(),
        line2: fLine2.trim() || undefined,
        city: fCity.trim() || undefined,
        state: fState.trim() || undefined,
        postalCode: fPostal.trim() || undefined,
        countryCode: (fCountry.trim() || 'ID').toUpperCase(),
        isDefaultShipping: fDefaultShipping,
        isDefaultBilling: fDefaultBilling,
      };

      if (editingAddressId) {
        await api.users.updateMyAddress(editingAddressId, payload);
      } else {
        await api.users.createMyAddress(payload);
      }
      showAddressModal = false;
      resetAddressForm();
      await load();
      toast.success($t('common.saved') || 'Saved');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Failed to save address'));
    } finally {
      savingAddress = false;
    }
  }

  function askDeleteAddress(id: string) {
    deleteAddressId = id;
    showDeleteAddress = true;
  }

  async function doDeleteAddress() {
    if (!deleteAddressId) return;
    deletingAddress = true;
    try {
      await api.users.deleteMyAddress(deleteAddressId);
      showDeleteAddress = false;
      deleteAddressId = null;
      await load();
      toast.success($t('common.deleted') || 'Deleted');
    } catch (e: any) {
      toast.error(String(e?.message || e || 'Failed to delete address'));
    } finally {
      deletingAddress = false;
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
        {$t('dashboard.locations.subtitle') ||
          'Your service locations. If something looks wrong, contact support.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn-primary" onclick={openCreateAddress} disabled={loading}>
        <Icon name="plus" size={16} />
        {$t('common.add') || 'Add'}
      </button>
      <button class="btn-secondary" onclick={load} disabled={loading}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
    </div>
  </div>

  <div class="summary-grid">
    <div class="summary card">
      <div class="summary-label">{$t('dashboard.locations.summary.total') || 'Total locations'}</div>
      <div class="summary-value">{totalLocations}</div>
    </div>
    <div class="summary card">
      <div class="summary-label">{$t('dashboard.locations.summary.customer') || 'Customer-linked'}</div>
      <div class="summary-value">{locations.length}</div>
    </div>
    <div class="summary card">
      <div class="summary-label">{$t('dashboard.locations.summary.profile') || 'Profile addresses'}</div>
      <div class="summary-value">{userAddresses.length}</div>
    </div>
  </div>

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
  {:else if locations.length === 0 && userAddresses.length === 0}
    <div class="empty card">
      <Icon name="map-pin" size={28} />
      <div class="empty-text">
        <div class="title">{$t('dashboard.locations.empty') || 'No locations yet.'}</div>
        <div class="sub">
          {$t('dashboard.locations.empty_hint') ||
            'Your admin has not linked your account to a customer location.'}
        </div>
      </div>
    </div>
  {:else}
    <div class="grid">
      {#each mergedLocations as loc (loc.id)}
        <div class="location card">
          <div class="top">
            <div class="badge">
              <Icon name="map-pin" size={16} />
              <span>{loc.source}</span>
            </div>
            {#if loc.isProfile}
              {@const addr = userAddresses.find((a) => a.id === loc.profileId)}
              <div class="row-actions">
                <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => addr && openEditAddress(addr)}>
                  <Icon name="edit" size={14} />
                </button>
                <button class="btn-icon danger" title={$t('common.delete') || 'Delete'} onclick={() => loc.profileId && askDeleteAddress(loc.profileId)}>
                  <Icon name="trash-2" size={14} />
                </button>
              </div>
            {/if}
          </div>
          <div class="name">{loc.title}</div>
          <div class="addr">{loc.address || '—'}</div>
          {#if loc.notes}
            <div class="notes">{loc.notes}</div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<Modal
  show={showAddressModal}
  title={editingAddressId ? ($t('common.edit') || 'Edit') : ($t('common.add') || 'Add')}
  onclose={() => (showAddressModal = false)}
>
  <div class="form">
    <div class="grid2">
      <label><span>Label</span><input class="input" bind:value={fLabel} /></label>
      <label><span>Recipient</span><input class="input" bind:value={fRecipient} /></label>
    </div>
    <div class="grid2">
      <label><span>Phone</span><input class="input" bind:value={fPhone} /></label>
      <label>
        <span>Country</span>
        <select class="input" bind:value={fCountry}>
          <option value="ID">ID (Indonesia)</option>
          <option value="US">US (United States)</option>
        </select>
      </label>
    </div>
    <label><span>Address line 1</span><input class="input" bind:value={fLine1} /></label>
    <label><span>Address line 2</span><input class="input" bind:value={fLine2} /></label>
    <div class="grid3">
      <label><span>City</span><input class="input" bind:value={fCity} /></label>
      <label><span>State</span><input class="input" bind:value={fState} /></label>
      <label><span>Postal</span><input class="input" bind:value={fPostal} /></label>
    </div>
    <div class="checks">
      <label class="check"><input type="checkbox" bind:checked={fDefaultShipping} /> Default shipping</label>
      <label class="check"><input type="checkbox" bind:checked={fDefaultBilling} /> Default billing</label>
    </div>
    <div class="modal-actions">
      <button class="btn-secondary" onclick={() => (showAddressModal = false)}>{$t('common.cancel') || 'Cancel'}</button>
      <button class="btn-primary" onclick={saveAddress} disabled={savingAddress || !fLine1.trim()}>
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<ConfirmDialog
  show={showDeleteAddress}
  title={$t('common.delete') || 'Delete'}
  message={$t('common.confirm_delete') || 'Are you sure you want to delete this item?'}
  confirmText={$t('common.delete') || 'Delete'}
  cancelText={$t('common.cancel') || 'Cancel'}
  loading={deletingAddress}
  onconfirm={doDeleteAddress}
  oncancel={() => (showDeleteAddress = false)}
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

  .badge {
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

  .name {
    position: relative;
    font-size: 1.1rem;
    font-weight: 750;
    margin-bottom: 0.35rem;
  }

  .addr {
    position: relative;
    color: var(--text-secondary);
    line-height: 1.4;
    font-size: 0.95rem;
  }

  .notes {
    position: relative;
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

  .form {
    display: grid;
    gap: 0.75rem;
  }

  .grid2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.7rem;
  }

  .grid3 {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.7rem;
  }

  label > span {
    display: block;
    margin-bottom: 0.35rem;
    color: var(--text-secondary);
    font-size: 0.86rem;
  }

  .input {
    width: 100%;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 12px;
    padding: 0.6rem 0.7rem;
    outline: none;
  }

  .checks {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-secondary);
  }

  .modal-actions {
    margin-top: 0.3rem;
    display: flex;
    justify-content: flex-end;
    gap: 0.7rem;
  }

  @media (max-width: 980px) {
    .page-content {
      padding: 0.95rem;
    }

    .summary-grid {
      grid-template-columns: 1fr;
    }

    .grid2,
    .grid3 {
      grid-template-columns: 1fr;
    }

    .grid {
      grid-template-columns: 1fr;
    }
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: stretch;
    }
  }
</style>

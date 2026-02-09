<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { api, type UserAddress } from '$lib/api/client';
  import { t } from 'svelte-i18n';

  let { loading = false } = $props();

  let addresses = $state<UserAddress[]>([]);
  let isLoading = $state(false);
  let error = $state<string | null>(null);

  let showModal = $state(false);
  let isEdit = $state(false);
  let editingId = $state<string | null>(null);

  let confirmOpen = $state(false);
  let confirmTargetId = $state<string | null>(null);

  let form = $state({
    label: '',
    recipientName: '',
    phone: '',
    line1: '',
    line2: '',
    city: '',
    state: '',
    postalCode: '',
    countryCode: 'ID',
    isDefaultShipping: false,
    isDefaultBilling: false,
  });

  function resetForm() {
    form = {
      label: '',
      recipientName: '',
      phone: '',
      line1: '',
      line2: '',
      city: '',
      state: '',
      postalCode: '',
      countryCode: 'ID',
      isDefaultShipping: false,
      isDefaultBilling: false,
    };
  }

  async function load() {
    isLoading = true;
    error = null;
    try {
      addresses = await api.users.listMyAddresses();
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to load addresses');
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    load();
  });

  function openCreate() {
    isEdit = false;
    editingId = null;
    resetForm();
    showModal = true;
  }

  function openEdit(addr: UserAddress) {
    isEdit = true;
    editingId = addr.id;
    form = {
      label: addr.label || '',
      recipientName: addr.recipient_name || '',
      phone: addr.phone || '',
      line1: addr.line1 || '',
      line2: addr.line2 || '',
      city: addr.city || '',
      state: addr.state || '',
      postalCode: addr.postal_code || '',
      countryCode: addr.country_code || 'ID',
      isDefaultShipping: !!addr.is_default_shipping,
      isDefaultBilling: !!addr.is_default_billing,
    };
    showModal = true;
  }

  async function submit() {
    if (!form.line1.trim()) {
      error = $t('profile.addresses.validation.line1_required') || 'Address line is required.';
      return;
    }

    isLoading = true;
    error = null;
    try {
      if (isEdit && editingId) {
        await api.users.updateMyAddress(editingId, { ...form });
      } else {
        await api.users.createMyAddress({ ...form });
      }
      showModal = false;
      await load();
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to save address');
    } finally {
      isLoading = false;
    }
  }

  function askDelete(id: string) {
    confirmTargetId = id;
    confirmOpen = true;
  }

  async function confirmDelete() {
    if (!confirmTargetId) return;
    isLoading = true;
    error = null;
    try {
      await api.users.deleteMyAddress(confirmTargetId);
      await load();
    } catch (e: any) {
      error = String(e?.message || e || 'Failed to delete address');
    } finally {
      isLoading = false;
      confirmOpen = false;
      confirmTargetId = null;
    }
  }
</script>

<div class="card section fade-in-up">
  <div class="card-header">
    <div>
      <h2 class="card-title">{$t('profile.addresses.title') || 'Addresses'}</h2>
      <p class="card-subtitle">
        {$t('profile.addresses.subtitle') || 'Manage multiple addresses for future billing/shipping.'}
      </p>
    </div>

    <div class="header-actions">
      <button class="btn btn-secondary" type="button" onclick={load} disabled={isLoading || loading}>
        <Icon name="refresh" size={18} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      <button class="btn btn-primary" type="button" onclick={openCreate} disabled={isLoading || loading}>
        <Icon name="plus" size={18} />
        {$t('profile.addresses.add') || 'Add Address'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="alert alert-error">
      <Icon name="alert" size={18} />
      <span>{error}</span>
    </div>
  {/if}

  {#if isLoading}
    <div class="loading-row">
      <span class="spinner"></span>
      <span>{$t('common.loading') || 'Loading…'}</span>
    </div>
  {:else if addresses.length === 0}
    <div class="empty-state">
      <div class="empty-icon">
        <Icon name="map-pin" size={22} />
      </div>
      <div class="empty-text">
        <div class="empty-title">{$t('profile.addresses.empty_title') || 'No addresses yet'}</div>
        <div class="empty-subtitle">
          {$t('profile.addresses.empty_subtitle') || 'Add one so you can reuse it for shipping and billing.'}
        </div>
      </div>
      <button class="btn btn-primary" type="button" onclick={openCreate}>
        <Icon name="plus" size={18} />
        {$t('profile.addresses.add') || 'Add Address'}
      </button>
    </div>
  {:else}
    <div class="address-grid">
      {#each addresses as addr (addr.id)}
        <div class="address-card">
          <div class="address-head">
            <div class="address-title">
              <Icon name="map-pin" size={18} />
              <div class="title-text">
                <div class="label">{addr.label || $t('profile.addresses.untitled') || 'Untitled'}</div>
                <div class="meta">
                  {#if addr.is_default_shipping}
                    <span class="badge badge-blue">
                      {$t('profile.addresses.default_shipping') || 'Default Shipping'}
                    </span>
                  {/if}
                  {#if addr.is_default_billing}
                    <span class="badge badge-purple">
                      {$t('profile.addresses.default_billing') || 'Default Billing'}
                    </span>
                  {/if}
                </div>
              </div>
            </div>

            <div class="address-actions">
              <button class="icon-btn" type="button" onclick={() => openEdit(addr)} title={$t('common.edit') || 'Edit'}>
                <Icon name="edit" size={16} />
              </button>
              <button class="icon-btn danger" type="button" onclick={() => askDelete(addr.id)} title={$t('common.delete') || 'Delete'}>
                <Icon name="trash" size={16} />
              </button>
            </div>
          </div>

          <div class="address-body">
            {#if addr.recipient_name || addr.phone}
              <div class="row">
                <span class="dim">{$t('profile.addresses.recipient') || 'Recipient'}</span>
                <span class="strong">
                  {addr.recipient_name || '-'}
                  {#if addr.phone}
                    <span class="sep">·</span>
                    <span class="mono">{addr.phone}</span>
                  {/if}
                </span>
              </div>
            {/if}

            <div class="addr-lines">
              <div class="strong">{addr.line1}</div>
              {#if addr.line2}
                <div class="dim">{addr.line2}</div>
              {/if}
              <div class="dim">
                {(addr.city || '')}{#if addr.city && addr.state}, {/if}{(addr.state || '')}
                {#if addr.postal_code} {addr.postal_code}{/if}
              </div>
              <div class="dim mono">{addr.country_code}</div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<Modal
  bind:show={showModal}
  title={isEdit
    ? $t('profile.addresses.edit_title') || 'Edit Address'
    : $t('profile.addresses.add_title') || 'Add Address'}
>
  <form
    class="settings-form"
    onsubmit={(e) => {
      e.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <div class="form-group">
        <label class="form-label" for="label">{$t('profile.addresses.fields.label') || 'Label'}</label>
        <input id="label" class="form-input" type="text" bind:value={form.label} placeholder="Home, Office, etc" />
      </div>

      <div class="form-group">
        <label class="form-label" for="recipientName">{$t('profile.addresses.fields.recipient_name') || 'Recipient name'}</label>
        <input id="recipientName" class="form-input" type="text" bind:value={form.recipientName} />
      </div>

      <div class="form-group">
        <label class="form-label" for="phone">{$t('profile.addresses.fields.phone') || 'Phone'}</label>
        <input id="phone" class="form-input" type="text" bind:value={form.phone} />
      </div>

      <div class="form-group span-2">
        <label class="form-label" for="line1">{$t('profile.addresses.fields.line1') || 'Address line 1'}</label>
        <input id="line1" class="form-input" type="text" bind:value={form.line1} required />
      </div>

      <div class="form-group span-2">
        <label class="form-label" for="line2">{$t('profile.addresses.fields.line2') || 'Address line 2'}</label>
        <input id="line2" class="form-input" type="text" bind:value={form.line2} />
      </div>

      <div class="form-group">
        <label class="form-label" for="city">{$t('profile.addresses.fields.city') || 'City'}</label>
        <input id="city" class="form-input" type="text" bind:value={form.city} />
      </div>

      <div class="form-group">
        <label class="form-label" for="state">{$t('profile.addresses.fields.state') || 'State/Province'}</label>
        <input id="state" class="form-input" type="text" bind:value={form.state} />
      </div>

      <div class="form-group">
        <label class="form-label" for="postalCode">{$t('profile.addresses.fields.postal_code') || 'Postal code'}</label>
        <input id="postalCode" class="form-input" type="text" bind:value={form.postalCode} />
      </div>

      <div class="form-group">
        <label class="form-label" for="countryCode">{$t('profile.addresses.fields.country_code') || 'Country code'}</label>
        <select id="countryCode" class="form-input" bind:value={form.countryCode}>
          <option value="ID">ID (Indonesia)</option>
          <option value="US">US (United States)</option>
        </select>
      </div>
    </div>

    <div class="toggle-row">
      <label class="toggle">
        <input type="checkbox" bind:checked={form.isDefaultShipping} />
        <span>{$t('profile.addresses.fields.default_shipping') || 'Default shipping'}</span>
      </label>
      <label class="toggle">
        <input type="checkbox" bind:checked={form.isDefaultBilling} />
        <span>{$t('profile.addresses.fields.default_billing') || 'Default billing'}</span>
      </label>
    </div>

    <div class="form-actions">
      <button class="btn btn-secondary" type="button" onclick={() => (showModal = false)} disabled={isLoading}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn btn-primary" type="submit" disabled={isLoading}>
        {#if isLoading}
          <span class="spinner"></span>
          {$t('common.saving') || 'Saving…'}
        {:else}
          <Icon name="save" size={18} />
          {$t('common.save') || 'Save'}
        {/if}
      </button>
    </div>
  </form>
</Modal>

<ConfirmDialog
  bind:show={confirmOpen}
  title={$t('profile.addresses.delete_title') || 'Delete address?'}
  message={$t('profile.addresses.delete_message') || 'This address will be removed permanently.'}
  confirmText={$t('common.delete') || 'Delete'}
  onconfirm={confirmDelete}
/>

<style>
  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 2rem;
    box-shadow: var(--shadow-sm);
    transition: all 0.2s ease;
  }

  .card-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-subtle);
  }

  .card-title {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
    letter-spacing: -0.01em;
  }

  .card-subtitle {
    font-size: 0.875rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .loading-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    color: var(--text-secondary);
    padding: 1rem 0;
  }

  .empty-state {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.25rem;
    border: 1px dashed var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--bg-app);
  }

  .empty-icon {
    width: 42px;
    height: 42px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    background: rgba(99, 102, 241, 0.12);
    color: var(--color-primary);
    flex: 0 0 auto;
  }

  .empty-text {
    flex: 1;
    min-width: 0;
  }

  .empty-title {
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .empty-subtitle {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .address-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1rem;
  }

  .address-card {
    border-radius: var(--radius-lg);
    background: var(--bg-app);
    border: 1px solid var(--border-subtle);
    padding: 1rem;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }

  .address-card:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--border-subtle), var(--color-primary) 22%);
  }

  .address-head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .address-title {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    color: var(--text-primary);
  }

  .title-text .label {
    font-weight: 700;
    line-height: 1.2;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.35rem;
  }

  .badge {
    font-size: 0.75rem;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-weight: 700;
  }

  .badge-blue {
    background: rgba(59, 130, 246, 0.12);
    border-color: rgba(59, 130, 246, 0.35);
    color: #93c5fd;
  }

  .badge-purple {
    background: rgba(168, 85, 247, 0.12);
    border-color: rgba(168, 85, 247, 0.35);
    color: #e9d5ff;
  }

  .address-actions {
    display: flex;
    gap: 0.35rem;
    align-items: flex-start;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    color: var(--text-primary);
    border-color: var(--border-color);
  }

  .icon-btn.danger:hover {
    border-color: rgba(239, 68, 68, 0.6);
    color: rgba(239, 68, 68, 1);
  }

  .address-body {
    display: grid;
    gap: 0.75rem;
  }

  .row {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: baseline;
  }

  .addr-lines {
    display: grid;
    gap: 0.25rem;
  }

  .strong {
    color: var(--text-primary);
    font-weight: 700;
  }

  .dim {
    color: var(--text-secondary);
    font-size: 0.875rem;
  }

  .mono {
    font-variant-numeric: tabular-nums;
  }

  .sep {
    margin: 0 0.35rem;
    opacity: 0.7;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }

  .span-2 {
    grid-column: span 2;
  }

  .form-label {
    display: block;
    margin-bottom: 0.35rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .form-input {
    width: 100%;
    padding: 0.75rem 1rem;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    outline: none;
    transition: border-color 0.2s ease;
  }

  .form-input:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.18);
  }

  .toggle-row {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-subtle);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.25rem;
  }

  @media (max-width: 820px) {
    .card-header {
      flex-direction: column;
      align-items: stretch;
    }
    .header-actions {
      justify-content: flex-start;
    }
    .form-grid {
      grid-template-columns: 1fr;
    }
    .span-2 {
      grid-column: auto;
    }
  }
</style>

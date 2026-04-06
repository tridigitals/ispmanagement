<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import { t } from 'svelte-i18n';
  import {
    getInitialNextPoolFieldState,
    resolveNextPoolFieldValue,
    type IpPoolNextPoolFieldState,
  } from '$lib/utils/ipPoolNextPool';

  export type IpPoolFormModel = {
    name: string;
    ranges: string;
    next_pool: string;
    comment: string;
  };

  let {
    show = $bindable(false),
    loading = false,
    isEditing = false,
    pool = $bindable(),
    nextPoolOptions = [],
    onSubmit,
  } = $props<{
    show: boolean;
    loading: boolean;
    isEditing: boolean;
    pool: IpPoolFormModel;
    nextPoolOptions: string[];
    onSubmit: () => void;
  }>();

  let nextPoolState = $state<IpPoolNextPoolFieldState>({
    mode: 'select',
    selectedValue: '',
    manualValue: '',
  });

  $effect(() => {
    nextPoolState = getInitialNextPoolFieldState(nextPoolOptions, pool?.next_pool, pool?.name);
  });

  function submit() {
    pool.next_pool = resolveNextPoolFieldValue(nextPoolState) || '';
    onSubmit();
  }
</script>

<Modal
  bind:show
  title={isEditing
    ? $t('admin.network.routers.ip_pools.form.edit_title') || 'Edit IP Pool'
    : $t('admin.network.routers.ip_pools.form.create_title') || 'Add IP Pool'}
  width="640px"
>
  <form
    class="modal-form"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="grid two">
      <label>
        <span>{$t('admin.network.routers.ip_pools.columns.name') || 'Name'}</span>
        <input bind:value={pool.name} disabled={loading || isEditing} required />
      </label>
      <label>
        <span>{$t('admin.network.routers.ip_pools.columns.next') || 'Next pool'}</span>
        <select bind:value={nextPoolState.selectedValue} disabled={loading || nextPoolState.mode === 'manual'}>
          <option value="">{($t('admin.network.routers.ip_pools.form.no_next_pool') as string) || 'No next pool'}</option>
          {#each nextPoolOptions as option}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </label>
    </div>

    <label class="toggle-row">
      <input
        type="checkbox"
        checked={nextPoolState.mode === 'manual'}
        disabled={loading}
        onchange={(event) => {
          nextPoolState.mode = event.currentTarget.checked ? 'manual' : 'select';
        }}
      />
      <span>{$t('admin.network.routers.ip_pools.form.next_pool_manual') || 'Enter next pool manually'}</span>
    </label>

    {#if nextPoolState.mode === 'manual'}
      <label>
        <span>{$t('admin.network.routers.ip_pools.form.next_pool_manual_label') || 'Manual next pool'}</span>
        <input
          bind:value={nextPoolState.manualValue}
          disabled={loading}
          placeholder="pool-overflow"
        />
      </label>
    {/if}

    <label>
      <span>{$t('admin.network.routers.ip_pools.columns.ranges') || 'Ranges'}</span>
      <input bind:value={pool.ranges} disabled={loading} placeholder="10.10.10.10-10.10.10.200" />
    </label>

    <label>
      <span>{$t('admin.network.routers.ip_pools.form.comment') || 'Comment'}</span>
      <textarea bind:value={pool.comment} disabled={loading} rows="3"></textarea>
    </label>

    <p class="router-note">
      {$t('admin.network.routers.ip_pools.form.router_note') ||
        'Changes are applied directly to the selected router before the local mirror is refreshed.'}
    </p>

    <div class="modal-actions">
      <button class="btn ghost" type="button" onclick={() => (show = false)} disabled={loading}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn primary" type="submit" disabled={loading}>
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </form>
</Modal>

<style>
  .modal-form {
    display: grid;
    gap: 1rem;
  }

  .grid {
    display: grid;
    gap: 1rem;
  }

  .grid.two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  label {
    display: grid;
    gap: 0.45rem;
    color: var(--text-secondary);
    font-weight: 700;
  }

  input,
  select,
  textarea {
    width: 100%;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-input, var(--bg-card));
    color: var(--text-primary);
    padding: 0.8rem 0.95rem;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    font-weight: 600;
  }

  .toggle-row input[type='checkbox'] {
    width: auto;
  }

  .router-note {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.5;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    font-weight: 800;
    cursor: pointer;
  }

  .btn.primary {
    background: var(--color-primary, #2563eb);
    border-color: var(--color-primary, #2563eb);
    color: white;
  }

  @media (max-width: 720px) {
    .grid.two {
      grid-template-columns: 1fr;
    }
  }
</style>

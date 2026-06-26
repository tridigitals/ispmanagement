<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import { t } from 'svelte-i18n';

  export type PppProfileFormModel = {
    name: string;
    local_address: string;
    remote_address: string;
    rate_limit: string;
    dns_server: string;
    comment: string;
    only_one: boolean;
  };

  let {
    show = $bindable(false),
    loading = false,
    isEditing = false,
    profile = $bindable(),
    remotePoolOptions = [],
    onSubmit,
  } = $props<{
    show: boolean;
    loading: boolean;
    isEditing: boolean;
    profile: PppProfileFormModel;
    remotePoolOptions: string[];
    onSubmit: () => void;
  }>();
</script>

<Modal
  bind:show
  title={isEditing
    ? $t('admin.network.routers.ppp_profiles.form.edit_title') || 'Edit PPP Profile'
    : $t('admin.network.routers.ppp_profiles.form.create_title') || 'Add PPP Profile'}
  width="640px"
>
  <form
    class="modal-form"
    onsubmit={(event) => {
      event.preventDefault();
      onSubmit();
    }}
  >
    <div class="grid two">
      <label>
        <span>{$t('admin.network.routers.ppp_profiles.columns.name')}</span>
        <input bind:value={profile.name} disabled={loading || isEditing} required />
      </label>
      <label>
        <span>{$t('admin.network.routers.ppp_profiles.columns.rate')}</span>
        <input bind:value={profile.rate_limit} disabled={loading} placeholder="10M/10M" />
      </label>
    </div>

    <div class="grid two">
      <label>
        <span>{$t('admin.network.routers.ppp_profiles.columns.local')}</span>
        <input bind:value={profile.local_address} disabled={loading} placeholder="10.10.10.1" />
      </label>
      <label>
        <span>{$t('admin.network.routers.ppp_profiles.columns.remote')}</span>
        <select bind:value={profile.remote_address} disabled={loading}>
          <option value="">{($t('admin.network.routers.ppp_profiles.form.no_remote_pool') as string) || 'No remote pool'}</option>
          {#each remotePoolOptions as option}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </label>
    </div>

    <label>
      <span>{$t('admin.network.routers.ppp_profiles.columns.dns')}</span>
      <input bind:value={profile.dns_server} disabled={loading} placeholder="1.1.1.1,8.8.8.8" />
    </label>

    <label>
      <span>{$t('admin.network.routers.ppp_profiles.form.comment')}</span>
      <textarea bind:value={profile.comment} disabled={loading} rows="3"></textarea>
    </label>

    <label class="toggle-row">
      <input type="checkbox" bind:checked={profile.only_one} disabled={loading} />
      <span>{$t('admin.network.routers.ppp_profiles.form.only_one')}</span>
    </label>

    <div class="modal-actions">
      <button class="btn ghost" type="button" onclick={() => (show = false)} disabled={loading}>
        {$t('common.cancel')}
      </button>
      <button class="btn primary" type="submit" disabled={loading}>
        {isEditing
          ? $t('common.save') || 'Save'
          : $t('admin.network.routers.ppp_profiles.form.create_action') || 'Create profile'}
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

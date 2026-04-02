<script lang="ts">
  import Input from '$lib/components/ui/Input.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import { t } from 'svelte-i18n';

  let {
    show = $bindable(false),
    loading = false,
    mode = 'reveal',
    mappingLabel = '',
    maskedSecret = '',
    revealedSecret = '',
    secretDraft = $bindable(''),
    onGenerate,
    onSubmit,
  } = $props<{
    show: boolean;
    loading: boolean;
    mode: 'reveal' | 'rotate';
    mappingLabel: string;
    maskedSecret: string;
    revealedSecret: string;
    secretDraft: string;
    onGenerate: () => void;
    onSubmit: () => void;
  }>();
</script>

<Modal
  bind:show
  title={mode === 'rotate'
    ? $t('superadmin.radius.modals.secret.rotate_title') || 'Rotate shared secret'
    : $t('superadmin.radius.modals.secret.reveal_title') || 'Reveal shared secret'}
  width="560px"
>
  <div class="dialog">
    <div class="summary">
      <span class="label">{$t('superadmin.radius.modals.secret.mapping') || 'Mapping'}</span>
      <strong>{mappingLabel}</strong>
    </div>

    <div class="summary">
      <span class="label">{$t('superadmin.radius.modals.secret.masked') || 'Masked secret'}</span>
      <code>{maskedSecret || '••••••••'}</code>
    </div>

    {#if mode === 'reveal'}
      <div class="summary">
        <span class="label">{$t('superadmin.radius.modals.secret.revealed') || 'Revealed secret'}</span>
        <code>{revealedSecret || ($t('superadmin.radius.modals.secret.not_loaded') || 'Click reveal to load the secret')}</code>
      </div>
    {:else}
      <div class="secret-row">
        <Input
          label={$t('superadmin.radius.form.shared_secret') || 'Shared secret'}
          type="password"
          bind:value={secretDraft}
          placeholder={$t('superadmin.radius.form.shared_secret_placeholder') || 'Leave blank to auto-generate'}
          disabled={loading}
          showPasswordToggle
        />
        <button class="btn btn-secondary btn-inline" type="button" onclick={onGenerate} disabled={loading}>
          {$t('superadmin.radius.actions.generate_secret') || 'Generate'}
        </button>
      </div>
    {/if}

    <div class="actions">
      <button class="btn btn-secondary" type="button" onclick={() => (show = false)} disabled={loading}>
        {$t('common.close') || 'Close'}
      </button>
      <button class="btn btn-primary" type="button" onclick={onSubmit} disabled={loading}>
        {#if loading}<span class="spinner-sm"></span>{/if}
        {mode === 'rotate'
          ? $t('superadmin.radius.actions.rotate_secret') || 'Rotate secret'
          : $t('superadmin.radius.actions.reveal_secret') || 'Reveal secret'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .dialog {
    display: grid;
    gap: 1rem;
  }

  .summary {
    display: grid;
    gap: 0.25rem;
  }

  .label {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  code {
    display: inline-block;
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    word-break: break-all;
  }

  .secret-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.75rem;
    align-items: end;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .btn-inline {
    min-height: 44px;
  }

  .spinner-sm {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.35);
    border-top-color: white;
    border-radius: 999px;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 720px) {
    .secret-row {
      grid-template-columns: 1fr;
    }
  }
</style>

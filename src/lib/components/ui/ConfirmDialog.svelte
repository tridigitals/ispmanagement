<script lang="ts">
  import Icon from './Icon.svelte';
  import { t } from 'svelte-i18n';

  let {
    show = $bindable(false),
    title = 'Confirm Action',
    message = 'Are you sure you want to proceed?',
    confirmText = 'Confirm',
    cancelText = 'Cancel',
    type = 'danger',
    confirmationKeyword = '',
    loading = false,
    onconfirm,
    oncancel,
  } = $props<{
    show?: boolean;
    title?: string;
    message?: string;
    confirmText?: string;
    cancelText?: string;
    type?: 'danger' | 'warning' | 'info';
    confirmationKeyword?: string;
    loading?: boolean;
    onconfirm?: () => void;
    oncancel?: () => void;
  }>();

  let inputValue = $state('');

  let canConfirm = $derived(
    !loading && (!confirmationKeyword || inputValue === confirmationKeyword),
  );

  function handleConfirm() {
    if (!canConfirm) return;
    if (onconfirm) onconfirm();
    inputValue = ''; // Reset after confirm
  }

  function handleCancel() {
    show = false;
    if (oncancel) oncancel();
    inputValue = ''; // Reset after cancel
  }
</script>

<!--
  Dialog konfirmasi SELALU harus berada di atas modal yang membukanya.

  Sebelumnya komponen ini membungkus `Modal`, yang memakai z-index 100. Karena
  keduanya bernilai sama, hasilnya bergantung pada urutan render DOM — dan di
  dalam modal yang punya z-index sendiri lebih tinggi (mis. NotificationModal
  z=1200, dipakai tombol "Tandai semua dibaca" / "Hapus semua"), dialog ini
  tertutup di belakang induknya sehingga tombol Konfirmasi/Batal tak bisa diklik.

  Karena itu dialog ini memakai overlay sendiri dengan `--z-confirm` (120) yang
  dijamin berada di atas `--z-modal` (100), berapa pun z-index modal induknya.
  Struktur dan token-nya meniru `Modal` supaya tampilannya identik.
-->
{#if show}
  <div
    class="confirm-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && handleCancel()}
  >
    <div
      class="confirm-card"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onclick={(e) => e.stopPropagation()}
    >
      <div class="confirm-header">
        <h3>{title}</h3>
        <button class="close-btn" type="button" onclick={handleCancel} aria-label="Close">
          <Icon name="x" size={20} />
        </button>
      </div>

      <div class="confirm-body">
        <div class="confirm-content">
          <div class="icon-wrapper {type}">
            <Icon
              name={type === 'danger'
                ? 'alert-circle'
                : type === 'warning'
                  ? 'alert-triangle'
                  : 'info'}
              size={32}
            />
          </div>
          <p class="message">{message}</p>

          {#if confirmationKeyword}
            <div class="confirmation-input">
              <p class="instruction">
                {$t('components.confirm_dialog.instruction_prefix')}
                <strong>{confirmationKeyword}</strong>
                {$t('components.confirm_dialog.instruction_suffix')}
              </p>
              <input
                type="text"
                bind:value={inputValue}
                placeholder={$t('components.confirm_dialog.placeholder', {
                  values: { keyword: confirmationKeyword },
                }) || `Type ${confirmationKeyword} here`}
                class="confirm-input"
              />
            </div>
          {/if}
        </div>
      </div>

      <div class="confirm-footer">
        <div class="actions">
          <button class="btn btn-ghost" onclick={handleCancel} disabled={loading}>
            {cancelText}
          </button>
          <button class="btn btn-{type}" onclick={handleConfirm} disabled={!canConfirm}>
            {#if loading}
              <span class="spinner"></span>
            {/if}
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    /* SELALU di atas modal mana pun (--z-modal = 100). Lihat komentar di atas. */
    z-index: var(--z-confirm, 120);
  }

  .confirm-card {
    background: var(--bg-surface, #1e293b);
    color: var(--text-primary, #f2f4f8);
    width: 100%;
    max-width: 400px;
    max-height: 90vh;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    box-shadow: var(--shadow-md);
    display: flex;
    flex-direction: column;
  }

  .confirm-header {
    padding: 1.25rem 1.25rem 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
  }

  .confirm-header h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary, white);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary, #94a3b8);
    cursor: pointer;
    padding: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    transition: all 0.2s;
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary, #fff);
  }

  .confirm-body {
    padding: 1.25rem;
    overflow-y: auto;
  }

  .confirm-footer {
    padding: 1rem 1.25rem 1.25rem;
    border-top: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
  }

  .confirm-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 0.5rem 0;
  }

  .icon-wrapper {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 1rem;
  }

  .icon-wrapper.danger {
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    color: var(--color-danger);
  }

  .icon-wrapper.warning {
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
    color: var(--color-warning);
  }

  .icon-wrapper.info {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  .message {
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0;
  }

  .confirmation-input {
    margin-top: 1.5rem;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .instruction {
    font-size: 0.9rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .instruction strong {
    color: var(--text-primary);
    user-select: all;
  }

  .confirm-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-radius: 6px;
    text-align: center;
    font-size: 1rem;
  }

  .confirm-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    width: 100%;
    justify-content: center;
  }

  .btn {
    padding: 0.6rem 1.25rem;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
  }

  .btn-ghost:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-danger {
    background: var(--color-danger);
    color: var(--bg-app);
  }
  .btn-danger:hover {
    filter: brightness(0.95);
  }

  .btn-warning {
    background: var(--color-warning);
    color: var(--bg-app);
  }
  .btn-warning:hover {
    filter: brightness(0.95);
  }

  .btn-info {
    background: var(--color-primary);
    color: var(--bg-app);
  }
  .btn-info:hover {
    filter: brightness(0.95);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    filter: grayscale(0.5);
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border-color);
    border-top-color: var(--bg-app);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>

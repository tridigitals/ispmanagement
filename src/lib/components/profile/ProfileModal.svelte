<script lang="ts">
  import { user } from '$lib/stores/auth';
  import { closeProfileModal, profileModal } from '$lib/stores/profileModal';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ProfileSurface from './ProfileSurface.svelte';

  function requestClose() {
    closeProfileModal();
  }

  function handleBackdropClick() {
    if ($profileModal.locked) return;
    requestClose();
  }

  function handleDialogKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && !$profileModal.locked) {
      event.preventDefault();
      requestClose();
    }
  }
</script>

{#if $profileModal.open}
  <div
    class="profile-modal-backdrop"
    role="presentation"
    onclick={handleBackdropClick}
    onkeydown={handleDialogKeydown}
    tabindex="-1"
  >
    <div
      class="profile-modal-shell"
      role="dialog"
      aria-modal="true"
      aria-labelledby="profile-modal-title"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={handleDialogKeydown}
    >
      <div class="profile-modal-topbar">
        <div class="profile-modal-copy">
          <div class="profile-modal-kicker">{$user?.role || $t('profile.fallback.member') || 'Member'}</div>
          <h2 id="profile-modal-title">{$t('profile.title')}</h2>
        </div>
        <button
          class="profile-modal-close"
          type="button"
          onclick={requestClose}
          disabled={$profileModal.locked}
          aria-label={$t('common.close')}
          title={$t('common.close')}
        >
          <Icon name="x" size={18} />
        </button>
      </div>

      <div class="profile-modal-content">
        <ProfileSurface
          requestedTab={$profileModal.tab}
          twoFARequired={$profileModal.reason === '2fa_required'}
        />
      </div>
    </div>
  </div>
{/if}

<style>
  .profile-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1200;
    background: rgba(8, 12, 19, 0.66);
    display: flex;
    align-items: stretch;
    justify-content: center;
    padding: clamp(12px, 2vw, 24px);
  }

  .profile-modal-shell {
    width: min(1180px, 100%);
    max-height: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .profile-modal-topbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-subtle);
    background: color-mix(in srgb, var(--bg-surface) 85%, transparent);
  }

  .profile-modal-copy h2 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary);
  }

  .profile-modal-kicker {
    margin-bottom: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .profile-modal-close {
    width: 38px;
    height: 38px;
    border-radius: 12px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
  }

  .profile-modal-close:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .profile-modal-content {
    overflow: auto;
    min-height: 0;
  }

  :global(.profile-modal-content .page-container) {
    max-width: none;
    min-height: auto;
  }

  @media (max-width: 900px) {
    .profile-modal-backdrop {
      padding: 0;
    }

    .profile-modal-shell {
      width: 100%;
      height: 100%;
      max-height: none;
      border-radius: 0;
      border: 0;
    }

    .profile-modal-topbar {
      padding: 0.9rem 1rem;
    }
  }
</style>

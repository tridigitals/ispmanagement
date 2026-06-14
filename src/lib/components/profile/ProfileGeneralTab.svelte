<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';

  let { profileData = $bindable(), loading = false, initials, onSave, onAvatarUpload } = $props();

  let fileInput: HTMLInputElement;

  function handleAvatarClick() {
    fileInput?.click();
  }

  function handleFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    // Validate type
    if (!['image/jpeg', 'image/png', 'image/webp'].includes(file.type)) {
      alert('Format gambar tidak didukung. Gunakan JPEG, PNG, atau WebP.');
      return;
    }
    // Validate size (5MB)
    if (file.size > 5 * 1024 * 1024) {
      alert('Ukuran gambar maksimal 5MB.');
      return;
    }

    const reader = new FileReader();
    reader.onload = () => {
      const base64 = (reader.result as string).split(',')[1];
      onAvatarUpload?.(base64);
    };
    reader.readAsDataURL(file);
    input.value = '';
  }
</script>

<div class="card section fade-in-up">
  <div class="card-header">
    <div>
      <h2 class="card-title">
        {$t('profile.general.title')}
      </h2>
      <p class="card-subtitle">
        {$t('profile.general.subtitle')}
      </p>
    </div>
  </div>

  <div class="profile-header-edit">
    <div class="avatar-large-wrapper">
      <div class="avatar-large">
        {#if profileData.avatar_url}
          <img src={profileData.avatar_url} alt="Avatar" class="avatar-img" />
        {:else}
          {initials}
        {/if}
      </div>
      <button
        class="avatar-edit-btn"
        title={$t('profile.general.change_avatar') || 'Change Avatar'}
        type="button"
        onclick={handleAvatarClick}
      >
        <Icon name="camera" size={16} />
      </button>
      <input
        bind:this={fileInput}
        type="file"
        accept="image/jpeg,image/png,image/webp"
        class="hidden"
        onchange={handleFileSelect}
      />
    </div>
    <div class="profile-header-text">
      <h3>
        {profileData.name || $t('profile.general.your_name') || 'Your Name'}
      </h3>
      <p>
        {profileData.role || $t('profile.general.role_label') || 'Role'}
      </p>
    </div>
  </div>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      onSave();
    }}
    class="settings-form"
  >
    <div class="form-grid">
      <div class="form-group">
        <label class="form-label" for="full-name">{$t('profile.general.display_name')}</label>
        <div class="input-wrapper">
          <Icon name="user" size={18} class="input-icon" />
          <input
            type="text"
            id="full-name"
            class="form-input with-icon"
            value={profileData.name}
            disabled
          />
        </div>
      </div>

      <div class="form-group">
        <label class="form-label" for="email">{$t('profile.general.email_address')}</label>
        <div class="input-wrapper">
          <Icon name="mail" size={18} class="input-icon" />
          <input
            type="email"
            id="email"
            class="form-input with-icon"
            value={profileData.email}
            disabled
          />
        </div>
      </div>

      <div class="form-group">
        <label class="form-label" for="phone">{$t('profile.general.phone') || 'Phone'}</label>
        <div class="input-wrapper">
          <Icon name="phone" size={18} class="input-icon" />
          <input
            type="tel"
            id="phone"
            class="form-input with-icon"
            placeholder={$t('profile.general.phone_placeholder') || 'Enter phone number'}
            bind:value={profileData.phone}
          />
        </div>
      </div>
    </div>

    <div class="form-actions">
      <button type="submit" class="btn btn-primary" disabled={loading}>
        {#if loading}
          <span class="spinner"></span>
          {$t('profile.general.saving')}
        {:else}
          <Icon name="save" size={18} />
          {$t('profile.general.save_button')}
        {/if}
      </button>
    </div>
  </form>
</div>

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
    margin-bottom: 2rem;
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

  .profile-header-edit {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    margin-bottom: 2.5rem;
    padding: 1.5rem;
    background: var(--bg-app);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
  }

  .avatar-large-wrapper {
    position: relative;
  }

  .avatar-large {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    background: var(--bg-surface);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 2rem;
    font-weight: 700;
    border: 4px solid var(--bg-surface);
    box-shadow: 0 0 0 1px var(--border-subtle);
    overflow: hidden;
  }

  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .hidden {
    display: none;
  }

  .avatar-edit-btn {
    position: absolute;
    bottom: 0;
    right: 0;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
    box-shadow: var(--shadow-sm);
  }

  .avatar-edit-btn:hover {
    color: var(--color-primary);
    border-color: var(--color-primary);
  }

  .profile-header-text h3 {
    margin: 0 0 0.25rem 0;
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .profile-header-text p {
    margin: 0;
    color: var(--text-secondary);
    font-weight: 500;
    text-transform: capitalize;
  }

  .settings-form {
    max-width: 100%;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1.5rem;
  }

  .form-group {
    margin-bottom: 0;
  }

  .form-label {
    display: block;
    margin-bottom: 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .input-wrapper {
    position: relative;
  }

  .form-input {
    width: 100%;
    padding: 0.75rem 1rem;
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 0.95rem;
    transition: all 0.2s;
  }

  .form-input.with-icon {
    padding-left: 2.75rem;
  }

  :global(.input-icon) {
    position: absolute;
    left: 1rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-tertiary);
    pointer-events: none;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
    background: var(--bg-surface);
  }

  .form-actions {
    margin-top: 2.5rem;
    display: flex;
    justify-content: flex-end;
    border-top: 1px solid var(--border-subtle);
    padding-top: 1.5rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.6rem 1.25rem;
    border-radius: var(--radius-md);
    font-weight: 500;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid transparent;
  }

  .btn-primary {
    background: var(--color-primary);
    color: var(--bg-app);
    box-shadow: var(--shadow-sm);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-md);
  }

  .btn-primary:disabled {
    opacity: 0.7;
    cursor: not-allowed;
    box-shadow: none !important;
    transform: none !important;
  }

  .spinner {
    width: 1rem;
    height: 1rem;
    border: 2px solid var(--border-color);
    border-radius: 50%;
    border-top-color: var(--bg-app);
    animation: spin 0.8s linear infinite;
    display: inline-block;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .fade-in-up {
    animation: fadeInUp 0.4s ease-out;
  }

  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Responsive */
  @media (max-width: 768px) {
    .card {
      padding: 1.5rem;
    }

    .profile-header-edit {
      flex-direction: column;
      text-align: center;
      gap: 1rem;
    }

    .form-grid {
      grid-template-columns: 1fr;
    }

    .btn {
      width: 100%;
    }
  }
</style>

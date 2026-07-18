<script lang="ts">
  import { user } from '$lib/stores/auth';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { api } from '$lib/api/client';

  let name = $state('');
  let email = $state('');
  let phone = $state('');
  let twofaEnabled = $state(false);
  let notifEmail = $state(true);
  let notifSms = $state(false);
  let notifWhatsapp = $state(false);
  let notifPromo = $state(false);
  let language = $state('id');
  let saving = $state(false);
  let showToast = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');

  $effect(() => {
    const u = $user;
    if (u) {
      name = u.name || '';
      email = u.email || '';
      phone = u.phone || '';
      twofaEnabled = !!(u.two_factor_enabled || u.totp_enabled);
    }
  });

  function triggerToast(msg: string, type: 'success' | 'error' = 'success') {
    toastMessage = msg;
    toastType = type;
    showToast = true;
    setTimeout(() => (showToast = false), 3000);
  }

  async function saveProfile() {
    saving = true;
    try {
      await api.auth.updateMe({ name, email, phone: phone || undefined });
      triggerToast($t('profile.messages.profile_updated') || 'Profile updated');
    } catch (e: any) {
      triggerToast(e?.message || 'Failed to save', 'error');
    } finally {
      saving = false;
    }
  }

  async function toggle2FA() {
    try {
      if (twofaEnabled) {
        await api.auth.disable2FA('');
        twofaEnabled = false;
        triggerToast('2FA disabled');
      } else {
        await api.auth.enable2FA();
        twofaEnabled = true;
        triggerToast('2FA enabled');
      }
    } catch (e: any) {
      triggerToast(e?.message || '2FA toggle failed', 'error');
    }
  }

  async function changePassword() {
    triggerToast('Password change dialog — open modal', 'error');
  }

  function toggleNotif(key: string) {
    // ponytail: placeholder — wire to real notification prefs store when backend supports per-channel toggles
    if (key === 'email') notifEmail = !notifEmail;
    else if (key === 'sms') notifSms = !notifSms;
    else if (key === 'whatsapp') notifWhatsapp = !notifWhatsapp;
    else if (key === 'promo') notifPromo = !notifPromo;
    triggerToast(`${key} toggled (demo)`, 'success');
  }
</script>

<div class="page fade-in">
  <div class="page-head">
    <div class="page-head-text">
      <h1>{$t('profile.title') || 'Pengaturan'}</h1>
      <p class="page-sub">Profil, keamanan, dan preferensi akun</p>
    </div>
  </div>

  <div class="profile-bar">
    <div class="avatar">{$user?.name?.charAt(0) || 'U'}</div>
    <div class="profile-meta">
      <div class="profile-name">{$user?.name || 'User'}</div>
      <div class="profile-sub">
        <span class="role">{$user?.role || 'Customer'}</span>
        <span class="dot">·</span>
        <span>{$user?.email || ''}</span>
      </div>
    </div>
  </div>

  <div class="grid">
    <section class="panel">
      <h2 class="panel-title">
        <Icon name="user" size={16} />
        Informasi pribadi
      </h2>
      <div class="form-group">
        <label class="form-label" for="s-name">Nama</label>
        <input id="s-name" class="form-input" type="text" bind:value={name} placeholder="Nama lengkap" />
      </div>
      <div class="form-group">
        <label class="form-label" for="s-email">Email</label>
        <input id="s-email" class="form-input" type="email" bind:value={email} placeholder="email@contoh.com" />
      </div>
      <div class="form-group">
        <label class="form-label" for="s-phone">Telepon</label>
        <input id="s-phone" class="form-input" type="tel" bind:value={phone} placeholder="0812xxxx" />
      </div>
      <button class="btn btn-primary" type="button" onclick={saveProfile} disabled={saving}>
        <Icon name="save" size={14} />
        {saving
          ? $t('profile.general.saving') || 'Saving...'
          : $t('profile.general.save_button') || 'Simpan'}
      </button>
    </section>

    <section class="panel">
      <h2 class="panel-title">
        <Icon name="shield" size={16} />
        Keamanan
      </h2>
      <div class="row">
        <div>
          <div class="row-label">Two-Factor Authentication</div>
          <div class="row-sub">{twofaEnabled ? 'Aktif' : 'Nonaktif'}</div>
        </div>
        <button
          class="toggle"
          class:on={twofaEnabled}
          type="button"
          onclick={toggle2FA}
          aria-label="Toggle 2FA"
        ></button>
      </div>
      <button class="btn btn-ghost" type="button" onclick={changePassword}>
        <Icon name="lock" size={14} />
        Ganti password
      </button>
    </section>

    <section class="panel">
      <h2 class="panel-title">
        <Icon name="bell" size={16} />
        Notifikasi
      </h2>
      {#each [
        { key: 'email', label: 'Email', state: notifEmail, icon: 'mail' },
        { key: 'sms', label: 'SMS', state: notifSms, icon: 'smartphone' },
        { key: 'whatsapp', label: 'WhatsApp', state: notifWhatsapp, icon: 'message-circle' },
        { key: 'promo', label: 'Promo', state: notifPromo, icon: 'tag' },
      ] as item}
        <div class="row">
          <div class="row-left">
            <Icon name={item.icon} size={14} />
            <span>{item.label}</span>
          </div>
          <button
            class="toggle"
            class:on={item.state}
            type="button"
            onclick={() => toggleNotif(item.key)}
            aria-label="Toggle {item.label}"
          ></button>
        </div>
      {/each}
    </section>

    <section class="panel">
      <h2 class="panel-title">
        <Icon name="globe" size={16} />
        Bahasa
      </h2>
      <div class="form-group">
        <label class="form-label" for="s-lang">Bahasa antarmuka</label>
        <select id="s-lang" class="form-select" bind:value={language}>
          <option value="id">Bahasa Indonesia</option>
          <option value="en">English</option>
        </select>
      </div>
    </section>
  </div>
</div>

{#if showToast}
  <div class="toast {toastType}">{toastMessage}</div>
{/if}

<style>
  .page {
    padding: clamp(1rem, 2.2vw, 1.75rem);
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .page-head h1 {
    font-size: clamp(1.25rem, 2.2vw, 1.45rem);
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
    color: var(--text-primary);
  }
  .page-sub {
    color: var(--text-secondary);
    font-size: 0.88rem;
    margin: 0.25rem 0 0;
  }

  .profile-bar {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding: 1rem 1.1rem;
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
  }
  .avatar {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--color-primary) 22%, transparent);
    color: var(--color-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 750;
    font-size: 1.15rem;
    flex-shrink: 0;
  }
  .profile-name {
    font-weight: 700;
    color: var(--text-primary);
    font-size: 1.05rem;
  }
  .profile-sub {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: center;
    font-size: 0.82rem;
    color: var(--text-secondary);
    margin-top: 0.15rem;
  }
  .role {
    color: var(--color-primary);
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    font-size: 0.72rem;
  }
  .dot {
    opacity: 0.5;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.85rem;
  }
  .panel {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 1.1rem 1.15rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .panel-title {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    margin: 0 0 0.25rem;
    padding-bottom: 0.65rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 0.92rem;
    font-weight: 650;
    color: var(--text-primary);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .form-label {
    font-size: 0.78rem;
    font-weight: 650;
    color: var(--text-secondary);
  }
  .form-input,
  .form-select {
    width: 100%;
    min-height: 42px;
    padding: 0.55rem 0.75rem;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(0, 0, 0, 0.2);
    color: var(--text-primary);
    font-size: 0.9rem;
  }
  .form-input:focus,
  .form-select:focus {
    outline: none;
    border-color: color-mix(in srgb, var(--color-primary) 45%, transparent);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-height: 44px;
  }
  .row-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-primary);
    font-size: 0.88rem;
  }
  .row-label {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .row-sub {
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-top: 0.1rem;
  }

  .toggle {
    width: 42px;
    height: 24px;
    border-radius: 999px;
    border: 0;
    background: rgba(255, 255, 255, 0.12);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
  }
  .toggle::after {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    transition: transform 0.15s ease;
  }
  .toggle.on {
    background: var(--color-primary);
  }
  .toggle.on::after {
    transform: translateX(18px);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem 0.95rem;
    border-radius: 8px;
    font-weight: 650;
    font-size: 0.88rem;
    cursor: pointer;
    border: none;
    min-height: 42px;
    width: fit-content;
  }
  .btn-primary {
    background: var(--color-primary);
    color: #fff;
  }
  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .toast {
    position: fixed;
    bottom: 1.25rem;
    right: 1.25rem;
    z-index: 50;
    padding: 0.75rem 1rem;
    border-radius: 10px;
    font-size: 0.88rem;
    font-weight: 600;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }
  .toast.success {
    background: color-mix(in srgb, var(--color-success) 20%, #111);
    color: var(--color-success);
    border: 1px solid color-mix(in srgb, var(--color-success) 35%, transparent);
  }
  .toast.error {
    background: color-mix(in srgb, var(--color-danger) 20%, #111);
    color: var(--color-danger);
    border: 1px solid color-mix(in srgb, var(--color-danger) 35%, transparent);
  }

  @media (max-width: 900px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 560px) {
    .btn {
      width: 100%;
      min-height: 44px;
    }
  }
</style>

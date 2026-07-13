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
  let language = $state('en');
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
    triggerToast(`${key} toggled (demo)`, 'success');
  }
</script>

<div class="settings-page">
  <!-- Profile Header -->
  <section class="hero-card profile-hero">
    <div class="profile-avatar">
      <span class="avatar-text">{$user?.name?.charAt(0) || 'U'}</span>
    </div>
    <div class="profile-info">
      <h1 class="profile-name">{$user?.name || 'User'}</h1>
      <span class="profile-role">{$user?.role || 'Customer'}</span>
      <span class="profile-email">{$user?.email || ''}</span>
    </div>
  </section>

  <div class="section-header"><h2>Pengaturan Akun</h2></div>

  <div class="settings-grid">
    <!-- Informasi Pribadi -->
    <div class="summary-card">
      <h3 class="card-title">
        <Icon name="user" size={16} />
        Informasi Pribadi
      </h3>
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
      <button class="btn btn-primary" onclick={saveProfile} disabled={saving}>
        <Icon name="save" size={14} />
        {saving ? ($t('profile.general.saving') || 'Saving...') : ($t('profile.general.save_button') || 'Save Changes')}
      </button>
    </div>

    <!-- Keamanan -->
    <div class="summary-card">
      <h3 class="card-title">
        <Icon name="shield" size={16} />
        Keamanan
      </h3>
      <div class="flex-between" style="padding: .5rem 0;">
        <div>
          <div class="form-label" style="margin-bottom:0;">Two-Factor Authentication (2FA)</div>
          <span style="font-size:.78rem;color:var(--text-tertiary);">
            {twofaEnabled ? 'Enabled' : 'Disabled'}
          </span>
        </div>
        <button
          class="toggle-switch"
          class:on={twofaEnabled}
          onclick={toggle2FA}
          aria-label="Toggle 2FA"
        ></button>
      </div>
      <div style="margin-top:1rem;">
        <button class="btn btn-secondary" onclick={changePassword}>
          <Icon name="lock" size={14} />
          Ganti Password
        </button>
      </div>
    </div>

    <!-- Notifikasi -->
    <div class="summary-card">
      <h3 class="card-title">
        <Icon name="bell" size={16} />
        Notifikasi
      </h3>
      {#each [
        { key: 'email', label: 'Email', state: notifEmail, icon: 'mail' },
        { key: 'sms', label: 'SMS', state: notifSms, icon: 'smartphone' },
        { key: 'whatsapp', label: 'WhatsApp', state: notifWhatsapp, icon: 'message-circle' },
        { key: 'promo', label: 'Promo', state: notifPromo, icon: 'tag' },
      ] as item}
        <div class="flex-between" style="padding: .45rem 0;">
          <div class="flex-between" style="gap:.5rem;justify-content:flex-start;">
            <Icon name={item.icon} size={14} color="var(--text-secondary)" />
            <span style="font-size:.85rem;">{item.label}</span>
          </div>
          <button
            class="toggle-switch"
            class:on={item.state}
            onclick={() => toggleNotif(item.key)}
            aria-label="Toggle {item.label}"
          ></button>
        </div>
      {/each}
    </div>

    <!-- Bahasa & Tampilan -->
    <div class="summary-card">
      <h3 class="card-title">
        <Icon name="globe" size={16} />
        Bahasa & Tampilan
      </h3>
      <div class="form-group">
        <label class="form-label" for="s-lang">Bahasa</label>
        <select id="s-lang" class="form-select" bind:value={language}>
          <option value="en">English</option>
          <option value="id">Bahasa Indonesia</option>
        </select>
      </div>
    </div>
  </div>
</div>

{#if showToast}
  <div class="toast {toastType}">{toastMessage}</div>
{/if}

<style>
  .settings-page {
    max-width: var(--content-max-width, 1400px);
  }

  /* Profile Hero */
  .profile-hero {
    display: flex; align-items: center; gap: 1.25rem;
    margin-bottom: 1.5rem; padding: 1.5rem;
  }
  .profile-avatar {
    width: 64px; height: 64px; border-radius: 50%;
    background: linear-gradient(135deg, var(--color-primary), #5b6cf0);
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0;
  }
  .avatar-text {
    font-size: 1.6rem; font-weight: 750; color: #fff;
  }
  .profile-info {
    display: flex; flex-direction: column; gap: .2rem;
    position: relative; z-index: 1;
  }
  .profile-name {
    font-size: 1.4rem; font-weight: 750; color: var(--text-primary); margin: 0;
  }
  .profile-role {
    font-size: .78rem; color: var(--color-primary); font-weight: 600;
    text-transform: uppercase; letter-spacing: .04em;
  }
  .profile-email {
    font-size: .85rem; color: var(--text-secondary); margin-top: .15rem;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
  }

  @media (max-width: 768px) {
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }

  .card-title {
    display: flex;
    align-items: center;
    gap: .5rem;
    font-size: .95rem;
    font-weight: 650;
    color: var(--text-primary);
    margin-bottom: 1rem;
    padding-bottom: .6rem;
    border-bottom: 1px solid rgba(255,255,255,.06);
  }
</style>

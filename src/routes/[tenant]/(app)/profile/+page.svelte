<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { token, user } from '$lib/stores/auth';
  import { theme } from '$lib/stores/theme';
  import { appSettings } from '$lib/stores/settings';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MobileFabMenu from '$lib/components/ui/MobileFabMenu.svelte';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';
  import {
    preferences,
    loadPreferences,
    updatePreference,
    subscribePush,
    unsubscribePush,
    sendTestNotification,
    checkSubscription,
    pushEnabled,
  } from '$lib/stores/notifications';

  // New Modular Components
  import ProfileGeneralTab from '$lib/components/profile/ProfileGeneralTab.svelte';
  import ProfileSecurityTab from '$lib/components/profile/ProfileSecurityTab.svelte';
  import ProfilePreferencesTab from '$lib/components/profile/ProfilePreferencesTab.svelte';
  import ProfileNotificationsTab from '$lib/components/profile/ProfileNotificationsTab.svelte';
  import ProfileAddressesTab from '$lib/components/profile/ProfileAddressesTab.svelte';

  let activeTab = $state('general');
  let loading = $state(false);
  let message = $state<{ type: '' | 'success' | 'error'; text: string }>({
    type: '',
    text: '',
  });

  // User Data State
  let profileData = $state({
    id: '',
    name: '',
    email: '',
    role: '',
  });

  // Password State
  let passwordData = $state({
    current: '',
    new: '',
    confirm: '',
  });

  // 2FA State
  let twoFactorData = $state({
    enabled: false,
    secret: '',
    qr: '',
    code: '',
    recoveryCodes: [] as string[],
    showSetup: false,
    showRecovery: false,
    disableCode: '',
  });

  // Trusted Devices State
  let trustedDevices = $state<any[]>([]);
  let loadingDevices = $state(true);

  let isDesktop = $state(false);

  let tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  let tenantPrefix = $derived(tenantCtx.tenantPrefix);

  let notificationCategories = $derived([
    {
      id: 'system',
      icon: 'server',
      label: $t('profile.notifications.categories.system.label') || 'System Updates',
      desc: $t('profile.notifications.categories.system.desc') || 'Maintenance & announcements',
    },
    {
      id: 'support',
      icon: 'life-buoy',
      label: $t('profile.notifications.categories.support.label') || 'Support Tickets',
      desc: $t('profile.notifications.categories.support.desc') || 'New tickets & replies',
    },
    {
      id: 'team',
      icon: 'users',
      label: $t('profile.notifications.categories.team.label') || 'Team Activity',
      desc: $t('profile.notifications.categories.team.desc') || 'Member changes & invites',
    },
    {
      id: 'payment',
      icon: 'credit-card',
      label: $t('profile.notifications.categories.payment.label') || 'Billing',
      desc: $t('profile.notifications.categories.payment.desc') || 'Invoices & subscriptions',
    },
    {
      id: 'security',
      icon: 'shield',
      label: $t('profile.notifications.categories.security.label') || 'Security',
      desc:
        $t('profile.notifications.categories.security.desc') || 'Login alerts & password changes',
    },
  ]);

  let pushPermission = $state<'default' | 'granted' | 'denied'>('default');

  let policy = $derived(
    $appSettings.auth || {
      password_min_length: 8,
      password_require_uppercase: true,
      password_require_number: true,
      password_require_special: false,
    },
  );

  // Initial Load
  onMount(async () => {
    void appSettings.init();
    isDesktop = !!(window as any).__TAURI_INTERNALS__;

    if ($user) {
      profileData = {
        id: $user.id,
        name: $user.name,
        email: $user.email,
        role: $user.role,
      };
      twoFactorData.enabled = $user.two_factor_enabled || false;
      loadTrustedDevices();
    }

    loadPreferences();
    checkSubscription();

    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('2fa_required') === 'true') {
      activeTab = 'security';
      showMessage(
        'error',
        $t('profile.messages.twofa_required') ||
          'Your organization requires Two-Factor Authentication.',
      );
    } else {
      const tab = urlParams.get('tab');
      if (tab && ['general', 'security', 'preferences', 'notifications', 'addresses'].includes(tab)) {
        activeTab = tab;
      } else {
        const saved = localStorage.getItem('profile.activeTab');
        if (saved && ['general', 'security', 'preferences', 'notifications', 'addresses'].includes(saved)) {
          activeTab = saved;
        }
      }
    }
  });

  $effect(() => {
    if (typeof window === 'undefined') return;
    localStorage.setItem('profile.activeTab', activeTab);
  });

  function showMessage(type: 'success' | 'error', text: string) {
    message = { type, text };
    setTimeout(() => (message = { type: '', text: '' }), 4000);
  }

  async function loadTrustedDevices() {
    loadingDevices = true;
    try {
      trustedDevices = await api.auth.listTrustedDevices();
    } catch (e: any) {
      showMessage('error', 'Failed to load trusted devices: ' + e.toString());
    } finally {
      loadingDevices = false;
    }
  }

  // --- Actions ---

  async function saveProfile() {
    if (!$token) return;
    loading = true;
    try {
      await api.users.update(profileData.id, {
        name: profileData.name,
        email: profileData.email,
      });
      user.update((u) => (u ? { ...u, name: profileData.name, email: profileData.email } : null));
      showMessage('success', $t('profile.messages.profile_updated'));
    } catch (error: any) {
      showMessage('error', error.toString() || $t('profile.messages.update_failed'));
    } finally {
      loading = false;
    }
  }

  function validatePassword(pwd: string): string | null {
    if (pwd.length < policy.password_min_length) {
      return $t('auth.validation.min_length', {
        values: { length: policy.password_min_length },
      });
    }
    if (policy.password_require_uppercase && !/[A-Z]/.test(pwd))
      return $t('auth.validation.require_uppercase');
    if (policy.password_require_number && !/[0-9]/.test(pwd))
      return $t('auth.validation.require_number');
    if (policy.password_require_special && !/[!@#$%^&*()_+\-=[\]{}|;:',.<>?/`~]/.test(pwd))
      return $t('auth.validation.require_special');
    return null;
  }

  async function changePassword() {
    if (!$token) return;
    if (passwordData.new !== passwordData.confirm) {
      showMessage('error', $t('profile.messages.password_mismatch'));
      return;
    }
    const policyError = validatePassword(passwordData.new);
    if (policyError) {
      showMessage('error', policyError);
      return;
    }
    loading = true;
    try {
      await api.auth.changePassword($token, passwordData.current, passwordData.new);
      showMessage('success', $t('profile.messages.password_changed'));
      passwordData = { current: '', new: '', confirm: '' };
    } catch (error: any) {
      showMessage('error', error.toString() || $t('profile.messages.change_password_failed'));
    } finally {
      loading = false;
    }
  }

  // 2FA Actions
  let setupMethod = $state<'totp' | 'email'>('totp');

  async function start2FA(method: 'totp' | 'email') {
    setupMethod = method;
    loading = true;
    try {
      if (method === 'totp') {
        const { secret, qr } = await api.auth.enable2FA();
        twoFactorData.secret = secret;
        twoFactorData.qr = qr.replace(/\s/g, '');
      } else {
        await api.auth.requestEmail2FASetup();
      }
      twoFactorData.showSetup = true;
    } catch (error: any) {
      showMessage('error', error.toString());
    } finally {
      loading = false;
    }
  }

  async function verify2FA() {
    loading = true;
    try {
      if (setupMethod === 'totp') {
        const { recovery_codes } = await api.auth.verify2FASetup(
          twoFactorData.secret,
          twoFactorData.code,
        );
        twoFactorData.recoveryCodes = recovery_codes;
      } else {
        await api.auth.verifyEmail2FASetup(twoFactorData.code);
        twoFactorData.recoveryCodes = [];
      }
      twoFactorData.enabled = true;
      twoFactorData.showSetup = false;
      if (twoFactorData.recoveryCodes.length > 0) {
        twoFactorData.showRecovery = true;
      } else {
        showMessage('success', 'Two-factor authentication enabled!');
      }
      user.update((u) =>
        u
          ? {
              ...u,
              two_factor_enabled: true,
              preferred_2fa_method: setupMethod,
            }
          : null,
      );
    } catch (error: any) {
      showMessage('error', error.toString());
    } finally {
      loading = false;
    }
  }

  async function disable2FA() {
    loading = true;
    try {
      await api.auth.disable2FA(twoFactorData.disableCode);
      twoFactorData.enabled = false;
      twoFactorData.disableCode = '';
      user.update((u) => (u ? { ...u, two_factor_enabled: false } : null));
      showMessage('success', 'Two-factor authentication disabled.');
    } catch (error: any) {
      showMessage('error', error.toString());
    } finally {
      loading = false;
    }
  }

  let disableOtpSending = $state(false);
  let disableOtpSent = $state(false);

  async function sendDisableEmailOtp() {
    disableOtpSending = true;
    try {
      await api.auth.request2FADisableCode();
      disableOtpSent = true;
      showMessage('success', 'Verification code sent to your email.');
    } catch (error: any) {
      showMessage('error', error.toString());
    } finally {
      disableOtpSending = false;
    }
  }

  async function change2FAMethod(method: 'totp' | 'email') {
    if (!$user?.two_factor_enabled) return;
    loading = true;
    try {
      await api.auth.set2FAPreference(method);
      user.update((u) => (u ? { ...u, preferred_2fa_method: method } : null));
      showMessage('success', '2FA method updated.');
    } catch (error: any) {
      showMessage('error', error.toString());
    } finally {
      loading = false;
    }
  }

  async function handleRevokeDevice(device: any) {
    loading = true;
    try {
      await api.auth.revokeTrustedDevice(device.id);
      showMessage('success', 'Device access has been revoked.');
      loadTrustedDevices();
    } catch (error: any) {
      showMessage('error', 'Failed to revoke device: ' + error.toString());
    } finally {
      loading = false;
    }
  }

  // Initials
  let initials = $derived(
    profileData.name
      ? profileData.name
          .split(' ')
          .map((n) => n[0])
          .slice(0, 2)
          .join('')
          .toUpperCase()
      : 'U',
  );

  let tabs = $derived([
    { id: 'general', label: $t('profile.tabs.general'), icon: 'profile' },
    { id: 'security', label: $t('profile.tabs.security'), icon: 'lock' },
    { id: 'addresses', label: $t('profile.tabs.addresses') || 'Addresses', icon: 'map-pin' },
    {
      id: 'preferences',
      label: $t('profile.tabs.preferences'),
      icon: 'settings',
    },
    {
      id: 'notifications',
      label: $t('profile.tabs.notifications') || 'Notifications',
      icon: 'bell',
    },
  ]);
</script>

<div class="page-container fade-in">
  <div class="header-section">
    <h1 class="page-title">{$t('profile.title')}</h1>
    <p class="page-subtitle">{$t('profile.subtitle')}</p>
  </div>

  {#if message.text}
    <div class="alert alert-{message.type} slide-in">
      <Icon name={message.type === 'success' ? 'check' : 'alert'} size={20} />
      <span>{message.text}</span>
    </div>
  {/if}

  <div class="layout-grid">
    <!-- Sidebar Navigation -->
    <aside class="sidebar">
      <div class="user-mini-profile">
        <div class="avatar-circle">{initials}</div>
        <div class="user-info">
          <span class="name">{profileData.name || $t('profile.fallback.user') || 'User'}</span>
          <span class="role">{profileData.role || $t('profile.fallback.member') || 'Member'}</span>
        </div>
      </div>

      <nav class="nav-menu">
        {#each tabs as tab}
          <button
            class="nav-item {activeTab === tab.id ? 'active' : ''}"
            onclick={() => (activeTab = tab.id)}
          >
            <Icon name={tab.icon} size={18} />
            <span>{tab.label}</span>
          </button>
        {/each}
      </nav>
    </aside>

    <!-- Mobile FAB & Menu -->
    <MobileFabMenu items={tabs} bind:activeTab title={$t('profile.title')} />

    <!-- Main Content Area -->
    <main class="content-area">
      {#if activeTab === 'general'}
        <ProfileGeneralTab bind:profileData {loading} {initials} onSave={saveProfile} />
      {/if}

      {#if activeTab === 'security'}
        <ProfileSecurityTab
          user={$user}
          bind:passwordData
          bind:twoFactorData
          {trustedDevices}
          {loading}
          {loadingDevices}
          {policy}
          bind:setupMethod
          onChangePassword={changePassword}
          onStart2FA={start2FA}
          onVerify2FA={verify2FA}
          onDisable2FA={disable2FA}
          onSendDisableEmailOtp={sendDisableEmailOtp}
          onChange2FAMethod={change2FAMethod}
          onRevokeDevice={handleRevokeDevice}
          {disableOtpSending}
          {disableOtpSent}
        />
      {/if}

      {#if activeTab === 'addresses'}
        <ProfileAddressesTab {loading} />
      {/if}

      {#if activeTab === 'preferences'}
        <ProfilePreferencesTab theme={$theme} onToggleTheme={() => theme.toggle()} />
      {/if}

      {#if activeTab === 'notifications'}
        <ProfileNotificationsTab
          {notificationCategories}
          preferences={$preferences}
          pushEnabled={$pushEnabled}
          bind:pushPermission
          {isDesktop}
          {tenantPrefix}
          onUpdatePreference={updatePreference}
          onSubscribePush={subscribePush}
          onUnsubscribePush={unsubscribePush}
          onSendTestNotification={sendTestNotification}
          {goto}
        />
      {/if}
    </main>
  </div>
</div>

<style>
  /* Layout & Containers */
  .page-container {
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
    min-height: 100%;
    box-sizing: border-box;
    overflow-x: hidden;
  }

  .header-section {
    margin-bottom: 2rem;
  }

  .page-title {
    font-size: 1.875rem;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.025em;
    margin: 0;
  }

  .page-subtitle {
    color: var(--text-secondary);
    font-size: 1rem;
    margin-top: 0.5rem;
  }

  .layout-grid {
    display: grid;
    grid-template-columns: 260px 1fr;
    gap: 2rem;
    align-items: start;
    overflow: hidden;
  }

  /* Main Content Area */
  .content-area {
    min-width: 0;
    overflow: hidden;
  }

  /* Sidebar */
  .sidebar {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1rem;
    position: sticky;
    top: 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .user-mini-profile {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid var(--border-subtle);
  }

  .avatar-circle {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--color-primary), var(--color-primary-hover));
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    font-size: 1.1rem;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    flex-shrink: 0;
  }

  .user-info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .user-info .name {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .user-info .role {
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
    margin-top: 2px;
  }

  .nav-menu {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.75rem 1rem;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.95rem;
    font-weight: 500;
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
    text-align: left;
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
    font-weight: 600;
  }

  /* Alerts */
  .alert {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    margin-bottom: 2rem;
    border-radius: var(--radius-md);
    font-size: 0.9rem;
    font-weight: 500;
  }

  .alert-success {
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.2);
    color: var(--color-success);
  }

  .alert-error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: var(--color-danger);
  }

  .fade-in {
    animation: fadeIn 0.4s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  /* Responsive Breakpoints */
  @media (max-width: 900px) {
    .layout-grid {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }

    .sidebar {
      display: none;
    }
  }

  @media (max-width: 600px) {
    .page-container {
      padding: 1.25rem;
    }

    .header-section {
      margin-bottom: 1.5rem;
    }

    .page-title {
      font-size: 1.5rem;
    }
  }
</style>

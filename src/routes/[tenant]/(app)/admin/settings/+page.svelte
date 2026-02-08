<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { user, isAdmin, can, getToken } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { goto } from '$app/navigation';
  import { locale, t, waitLocale } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MobileFabMenu from '$lib/components/ui/MobileFabMenu.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import type { Setting } from '$lib/api/client';
  import { toast } from 'svelte-sonner';
  import { get } from 'svelte/store';
  import { adminSettingsCache } from '$lib/stores/adminSettingsCache';

  let loading = $state(true);
  let saving = $state(false);
  let settings = $state<Record<string, Setting>>({});
  let localSettings = $state<Record<string, string>>({});
  let logoBase64 = $state<string | null>(null);
  let activeTab = $state('general');
  let hasChanges = $state(false);
  let isMobile = $state(false);

  // Tenant specific state
  let tenantInfo = $state<any>(null);
  let tenantChanges = $state<{ name?: string; customDomain?: string }>({});
  let customDomainAccess = $state(false);

  // Baseline snapshot for local reset (no network)
  let baselineLocalSettings = $state<Record<string, string>>({});
  let baselineLogoBase64 = $state<string | null>(null);
  let baselineTenantInfo = $state<any>(null);
  let baselineCustomDomainAccess = $state(false);

  // Categories configuration (i18n-aware)
  let categories = $derived.by(() => ({
    general: {
      label: $t('admin.settings.categories.general') || 'General',
      icon: 'app',
      keys: [
        'app_name',
        'app_description',
        'support_email',
        'default_locale',
        'currency_code',
        'app_logo_path',
      ],
    },
    branding: {
      // New Branding & Domain Tab
      label: $t('admin.settings.categories.branding') || 'Branding & Domain',
      icon: 'globe',
      keys: [], // Managed manually
    },
    security: {
      label: $t('admin.settings.categories.security') || 'Security',
      icon: 'shield',
      keys: [], // Managed manually
    },
    storage: {
      label: $t('admin.settings.categories.storage') || 'Storage',
      icon: 'database',
      keys: [
        'storage_driver',
        'storage_s3_bucket',
        'storage_s3_region',
        'storage_s3_endpoint',
        'storage_s3_access_key',
        'storage_s3_secret_key',
        'storage_s3_public_url',
      ],
    },
    email: {
      label: $t('admin.settings.categories.email') || 'Email',
      icon: 'mail',
      keys: [
        'email_provider',
        'email_smtp_host',
        'email_smtp_port',
        'email_smtp_username',
        'email_smtp_password',
        'email_smtp_encryption',
        'email_api_key',
        'email_from_address',
        'email_from_name',
        'email_webhook_url',
      ],
    },
    payment: {
      label: $t('admin.settings.categories.payment') || 'Payments',
      icon: 'credit-card',
      keys: [
        'payment_midtrans_enabled',
        'payment_midtrans_merchant_id',
        'payment_midtrans_client_key',
        'payment_midtrans_server_key',
        'payment_midtrans_is_production',
        'payment_manual_enabled',
        'payment_manual_instructions',
        'payment_manual_accounts',
      ],
    },
  }));

  let mobileMenuItems = $derived(
    Object.entries(categories).map(([id, cat]) => ({
      id,
      label: cat.label,
      icon: cat.icon,
    })),
  );

  onMount(async () => {
    if (!$isAdmin || !$can('read', 'settings')) {
      goto('/unauthorized');
      return;
    }

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 900px)');
      const sync = () => {
        isMobile = mq.matches;
      };
      sync();
      try {
        mq.addEventListener('change', sync);
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
      }
    }

    // Hydrate from cache to avoid loading flash
    const cacheKey = String($user?.tenant_id || $user?.tenant_slug || '');
    if (cacheKey) {
      const cached = get(adminSettingsCache)[cacheKey];
      if (cached?.fetchedAt) {
        loading = false;
        buildLocalSettingsFromData(
          cached.settings,
          cached.tenantInfo,
          cached.customDomainAccess,
          cached.logoBase64,
        );
        void loadSettings({ silent: true });
        return;
      }
    }

    await loadSettings();
  });

  let activeCategory = $derived(categories[activeTab as keyof typeof categories]);

  function buildLocalSettingsFromData(
    data: Setting[],
    tenant: any,
    access: boolean,
    logo: string | null,
  ) {
    // Map settings
    settings = data.reduce(
      (acc, curr) => {
        acc[curr.key] = curr;
        return acc;
      },
      {} as Record<string, Setting>,
    );

    tenantInfo = tenant;
    tenantChanges = {};
    customDomainAccess = access;

    localSettings = {};
    Object.values(categories).forEach((cat) => {
      cat.keys.forEach((key) => {
        let val = settings[key]?.value ?? '';
        if (key === 'storage_driver' && !val) val = 'system';
        if (key === 'currency_code') {
          val = (val || 'IDR').toUpperCase();
          if (val !== 'IDR' && val !== 'USD') val = 'IDR';
        }
        localSettings[key] = val;
      });
    });

    // Tenant locals
    localSettings['tenant_name'] = tenantInfo?.name || '';
    localSettings['custom_domain'] = tenantInfo?.custom_domain || '';
    localSettings['enforce_2fa'] = String(tenantInfo?.enforce_2fa ?? false);

    // Init Bank Accounts (from JSON string)
    loadBankAccounts();

    logoBase64 = logo;

    // Baseline snapshot for reset
    baselineLocalSettings = { ...localSettings };
    baselineLogoBase64 = logoBase64;
    baselineTenantInfo = tenantInfo ? { ...tenantInfo } : null;
    baselineCustomDomainAccess = customDomainAccess;

    hasChanges = false;
  }

  function recomputeHasChanges() {
    // Tenant changes
    const nameChanged = (localSettings['tenant_name'] || '') !== (baselineTenantInfo?.name || '');
    const domainChanged =
      (localSettings['custom_domain'] || '') !== (baselineTenantInfo?.custom_domain || '');
    const enforceChanged =
      String(localSettings['enforce_2fa'] || 'false') !==
      String(baselineTenantInfo?.enforce_2fa ?? false);

    // Setting changes (all keys across categories)
    const keys = new Set<string>();
    Object.values(categories).forEach((cat) => cat.keys.forEach((k) => keys.add(k)));

    let settingsChanged = false;
    for (const key of keys) {
      const baseVal = baselineLocalSettings[key] ?? '';
      const curVal = localSettings[key] ?? '';
      if (curVal !== baseVal) {
        settingsChanged = true;
        break;
      }
    }

    const logoChanged = (logoBase64 || '') !== (baselineLogoBase64 || '');

    hasChanges = nameChanged || domainChanged || enforceChanged || settingsChanged || logoChanged;
  }

  async function loadSettings(opts: { silent?: boolean } = {}) {
    try {
      if (!opts.silent) loading = true;

      const token = getToken() || undefined;

      // Use current logo from store (fast) while refreshing in background
      let logoStoreValue: string | null = null;
      appLogo.subscribe((v) => (logoStoreValue = v))();

      const [_, data, tenant] = await Promise.all([
        appLogo.refresh(token).catch(() => null),
        api.settings.getAll(),
        api.tenant.getSelf(),
      ]);

      const access = await api.plans
        .checkAccess(tenant.id, 'custom_domain')
        .catch(() => ({ has_access: false }) as any);

      // Pull refreshed logo again if available
      let logoAfter: string | null = null;
      appLogo.subscribe((v) => (logoAfter = v))();

      buildLocalSettingsFromData(
        data,
        tenant,
        Boolean(access?.has_access),
        logoAfter || logoStoreValue || null,
      );

      const key = String(
        tenant?.id || tenant?.slug || $user?.tenant_id || $user?.tenant_slug || 'default',
      );
      adminSettingsCache.update((m) => ({
        ...m,
        [key]: {
          settings: data,
          tenantInfo: tenant,
          customDomainAccess: Boolean(access?.has_access),
          logoBase64: logoAfter || logoStoreValue || null,
          fetchedAt: Date.now(),
        },
      }));
    } catch (error) {
      console.error(error);
      toast.error(get(t)('admin.settings.toasts.load_failed') || 'Failed to load settings');
    } finally {
      if (!opts.silent) loading = false;
    }
  }

  function handleChange(key: string, value: any) {
    localSettings[key] = String(value);

    // Check if tenant setting
    if (key === 'tenant_name' || key === 'custom_domain' || key === 'enforce_2fa') {
      const originalName = tenantInfo?.name || '';
      const originalDomain = tenantInfo?.custom_domain || '';
      const originalEnforce = tenantInfo?.enforce_2fa ?? false;

      if (key === 'tenant_name' && value !== originalName) tenantChanges.name = value;
      if (key === 'custom_domain' && value !== originalDomain) tenantChanges.customDomain = value;
      if (key === 'enforce_2fa' && Boolean(value) !== originalEnforce)
        (tenantChanges as any).enforce2fa = Boolean(value);

      // Revert if matches original
      if (key === 'tenant_name' && value === originalName) delete tenantChanges.name;
      if (key === 'custom_domain' && value === originalDomain) delete tenantChanges.customDomain;
      if (key === 'enforce_2fa' && Boolean(value) === originalEnforce)
        delete (tenantChanges as any).enforce2fa;

      // keep tenantChanges for save payload, but use full recompute for UI state
    } else {
      // handled by recomputeHasChanges
    }

    localSettings = { ...localSettings };
    recomputeHasChanges();
  }

  async function handleFileUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    const file = input.files[0];

    try {
      const reader = new FileReader();
      reader.onload = async (e) => {
        const base64 = e.target?.result as string;
        const base64Data = base64.split(',')[1];
        const path = await api.settings.uploadLogo(base64Data);

        localSettings['app_logo_path'] = path;
        appLogo.set(base64);
        logoBase64 = base64;
        recomputeHasChanges();
        toast.success(get(t)('admin.settings.toasts.logo_uploaded') || 'Logo uploaded');
      };
      reader.readAsDataURL(file);
    } catch (error) {
      toast.error(get(t)('admin.settings.toasts.logo_upload_failed') || 'Failed to upload logo');
    }
  }

  async function saveChanges() {
    saving = true;
    try {
      // Save Tenant Changes
      if (Object.keys(tenantChanges).length > 0) {
        await api.tenant.updateSelf(tenantChanges);
      }

      // Save App Settings
      if (activeTab !== 'branding') {
        const keysToSave = categories[activeTab as keyof typeof categories].keys;
        await Promise.all(
          keysToSave.map((key) => {
            if (key === 'app_logo_path') return Promise.resolve();
            const val = localSettings[key];
            if (val !== undefined && val !== settings[key]?.value) {
              // If locale changed, update immediately
              if (key === 'default_locale') {
                locale.set(val);
                // We don't await waitLocale here because it's inside map/all
                // But we can trigger a reload effect
              }
              return api.settings.upsert(key, val);
            }
          }),
        );
      }

      // If locale changed, ensure it's loaded
      if (localSettings['default_locale'] !== settings['default_locale']?.value) {
        await waitLocale();
      }

      await loadSettings();
      await appSettings.refresh();
      toast.success(get(t)('admin.settings.toasts.saved') || 'Settings saved');
    } catch (error: any) {
      toast.error(
        error.message || get(t)('admin.settings.toasts.save_failed') || 'Failed to save settings',
      );
    } finally {
      saving = false;
    }
  }

  function discardChanges() {
    // Reset to baseline snapshot (no network)
    localSettings = { ...baselineLocalSettings };
    tenantChanges = {};
    customDomainAccess = baselineCustomDomainAccess;
    tenantInfo = baselineTenantInfo ? { ...baselineTenantInfo } : tenantInfo;
    logoBase64 = baselineLogoBase64;
    loadBankAccounts();
    recomputeHasChanges();
  }

  // Input Helpers
  const localeOptions = [
    { value: 'en', label: 'English (US)' },
    { value: 'id', label: 'Bahasa Indonesia (ID)' },
  ];
  const currencyCodeOptions = [
    { value: 'IDR', label: 'IDR (Indonesian Rupiah)' },
    { value: 'USD', label: 'USD (US Dollar)' },
  ];
  const storageOptions = [
    { value: 'system', label: 'System Default (Managed)' },
    { value: 's3', label: 'AWS S3' },
    { value: 'r2', label: 'Cloudflare R2' },
  ];
  const emailProviderOptions = [
    { value: 'smtp', label: 'SMTP' },
    { value: 'resend', label: 'Resend API' },
  ];
  const smtpEncryptionOptions = [
    { value: 'starttls', label: 'STARTTLS' },
    { value: 'tls', label: 'TLS/SSL' },
    { value: 'none', label: 'None' },
  ];

  function getLabel(key: string) {
    return key.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase());
  }

  // Test Email State
  let testEmailAddress = $state('');
  let sendingTestEmail = $state(false);
  let testingSmtp = $state(false);

  // Bank Account Management State
  let bankAccounts = $state<any[]>([]);
  let newBank = $state({
    bank_name: '',
    account_number: '',
    account_holder: '',
  });
  let showAddBank = $state(false);

  // Sync bankAccounts state with localSettings JSON string
  function loadBankAccounts() {
    try {
      const json = localSettings['payment_manual_accounts'];
      bankAccounts = json ? JSON.parse(json) : [];
    } catch (e) {
      bankAccounts = [];
    }
  }

  function addBankAccount() {
    if (!newBank.bank_name || !newBank.account_number || !newBank.account_holder) return;

    bankAccounts = [...bankAccounts, { ...newBank, id: crypto.randomUUID() }];
    newBank = { bank_name: '', account_number: '', account_holder: '' };
    showAddBank = false;

    // Update settings string
    handleChange('payment_manual_accounts', JSON.stringify(bankAccounts));
  }

  function removeBankAccount(id: string) {
    bankAccounts = bankAccounts.filter((b) => b.id !== id);
    handleChange('payment_manual_accounts', JSON.stringify(bankAccounts));
  }

  async function sendTestEmail() {
    if (!testEmailAddress) return;
    sendingTestEmail = true;
    try {
      const result = await api.settings.sendTestEmail(testEmailAddress);
      toast.success(result);
    } catch (error: any) {
      toast.error(error.message || 'Failed to send test email');
    } finally {
      sendingTestEmail = false;
    }
  }

  async function testSmtpConnection() {
    testingSmtp = true;
    try {
      const result = await api.settings.testSmtpConnection();
      toast.success(
        `${result.message} (${result.host}:${result.port}, ${result.encryption}, ${result.duration_ms}ms)`,
      );
    } catch (error: any) {
      toast.error(
        error.message || $t('admin.settings.email.smtp_test.failed') || 'SMTP test failed',
      );
    } finally {
      testingSmtp = false;
    }
  }

  // Plan Features Helper
  function getPlanFeatures(slug: string) {
    switch (slug) {
      case 'free':
        return ['Community Support', 'Basic Analytics', 'Subdomain Only'];
      case 'pro':
        return ['Priority Support', 'Advanced Analytics', 'Custom Domain', 'Remove Branding'];
      case 'enterprise':
        return [
          '24/7 Dedicated Support',
          'Audit Logs',
          'Custom Domain',
          'SSO & Security',
          'API Access',
        ];
      default:
        return [];
    }
  }
</script>

<div class="page-container fade-in">
  <div class="layout-grid">
    <!-- Sidebar -->
    <aside class="sidebar card desktop-sidebar">
      <nav>
        {#each Object.entries(categories) as [id, cat]}
          <button
            class="nav-item {activeTab === id ? 'active' : ''}"
            onclick={() => {
              activeTab = id;
              discardChanges();
            }}
          >
            <span class="icon"><Icon name={cat.icon} size={18} /></span>
            {cat.label}
          </button>
        {/each}
      </nav>
    </aside>

    <main class="content">
      {#if loading}
        <div class="loading-state"><div class="spinner"></div></div>
      {:else}
        <div class="card section fade-in">
          <div class="card-header">
            <h2 class="card-title">
              {categories[activeTab as keyof typeof categories].label}
            </h2>
            <p class="card-subtitle">
              {$t('admin.settings.subtitle_dynamic', {
                values: {
                  tab: categories[activeTab as keyof typeof categories].label,
                },
              }) || `Manage your ${activeTab} settings`}
            </p>
          </div>

          <div class="settings-body">
            {#if activeTab === 'branding'}
              <!-- Tenant Branding -->
              <div class="setting-group">
                <label for="tenant-name"
                  >{$t('admin.settings.keys.organization_name') || 'Organization Name'}</label
                >
                <Input
                  id="tenant-name"
                  value={localSettings['tenant_name']}
                  oninput={(e: any) => handleChange('tenant_name', e.target.value)}
                />
              </div>

              <div class="setting-group">
                <label for="custom-domain"
                  >{$t('admin.settings.keys.custom_domain') || 'Custom Domain'}</label
                >
                {#if customDomainAccess}
                  <Input
                    id="custom-domain"
                    value={localSettings['custom_domain']}
                    oninput={(e: any) => handleChange('custom_domain', e.target.value)}
                    placeholder={$t('admin.settings.placeholders.custom_domain') ||
                      'e.g. app.yourcompany.com'}
                  />
                  <p class="help-text">
                    {$t('admin.settings.branding.custom_domain_help_prefix') ||
                      "Point your domain's CNAME record to"} <code>cname.tridigitals.com</code>
                    {$t('admin.settings.branding.custom_domain_help_suffix') ||
                      '(or configured alias).'}
                  </p>
                {:else}
                  <div class="upgrade-banner">
                    <div class="icon-box">
                      <Icon name="lock" size={20} />
                    </div>
                    <div class="text">
                      <h4>
                        {$t('admin.settings.branding.custom_domain_pro_title') ||
                          'Custom Domain is a Pro Feature'}
                      </h4>
                      <p>
                        {$t('admin.settings.branding.custom_domain_pro_desc') ||
                          'Upgrade your plan to use your own domain name.'}
                      </p>
                    </div>
                    <button
                      class="btn btn-primary btn-sm"
                      onclick={() => goto(`/${tenantInfo?.slug}/admin/subscription`)}
                    >
                      {$t('common.upgrade_plan') || 'Upgrade Plan'}
                    </button>
                  </div>
                  <Input
                    value={localSettings['custom_domain']}
                    disabled={true}
                    placeholder={$t('admin.settings.placeholders.locked') || 'Locked'}
                  />
                {/if}
              </div>
            {:else if activeTab === 'security'}
              <!-- Security Settings -->
              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>
                    {$t('admin.settings.sections.enforce_2fa') ||
                      'Enforce Two-Factor Authentication'}
                  </h3>
                  <p>
                    {$t('admin.settings.security.enforce_2fa_desc') ||
                      'Require all members of this organization to enable 2FA before accessing the dashboard.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['enforce_2fa'] === 'true'}
                    onchange={(e) => handleChange('enforce_2fa', e.currentTarget.checked)}
                  />
                  <span class="slider"></span>
                </label>
              </div>
            {:else if activeTab === 'storage'}
              <!-- Redesigned Storage Settings -->
              <div class="storage-settings">
                <span class="section-label"
                  >{$t('admin.settings.storage.select_provider') || 'Select Storage Provider'}</span
                >
                <div class="provider-grid">
                  {#each storageOptions as option}
                    <button
                      class="provider-card"
                      class:selected={localSettings['storage_driver'] === option.value}
                      onclick={() => handleChange('storage_driver', option.value)}
                    >
                      <div class="p-icon">
                        {#if option.value === 's3'}
                          <Icon name="cloud" size={24} />
                        {:else if option.value === 'r2'}
                          <Icon name="globe" size={24} />
                        {:else}
                          <Icon name="server" size={24} />
                        {/if}
                      </div>
                      <div class="p-info">
                        <span class="p-name">{option.label}</span>
                        <span class="p-desc">
                          {#if option.value === 's3'}
                            Scalable object storage by AWS.
                          {:else if option.value === 'r2'}
                            Zero egress fee storage by Cloudflare.
                          {:else}
                            Local disk storage (Default).
                          {/if}
                        </span>
                      </div>
                      <div class="p-check">
                        <Icon
                          name={localSettings['storage_driver'] === option.value
                            ? 'check-circle'
                            : 'circle'}
                          size={20}
                        />
                      </div>
                    </button>
                  {/each}
                </div>

                {#if localSettings['storage_driver'] === 's3' || localSettings['storage_driver'] === 'r2'}
                  <div class="config-panel fade-in">
                    <h3>
                      {$t('admin.settings.sections.configuration') || 'Configuration'}
                    </h3>
                    <div class="config-grid">
                      {#each categories['storage'].keys as key}
                        {#if key !== 'storage_driver'}
                          <div class="setting-item">
                            <label for={key}>{getLabel(key)}</label>
                            <div class="setting-control">
                              <Input
                                type={key.includes('secret') || key.includes('key')
                                  ? 'password'
                                  : 'text'}
                                value={localSettings[key]}
                                oninput={(e: any) => handleChange(key, e.target.value)}
                                placeholder={key.includes('region') ? 'e.g. us-east-1' : ''}
                              />
                            </div>
                          </div>
                        {/if}
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
            {:else if activeTab === 'email'}
              <!-- Redesigned Email Settings -->
              <div class="email-settings">
                <span class="section-label"
                  >{$t('admin.settings.email.provider_label') || 'Email Delivery Provider'}</span
                >
                <div class="provider-grid">
                  {#each emailProviderOptions as option}
                    <button
                      class="provider-card"
                      class:selected={localSettings['email_provider'] === option.value}
                      onclick={() => handleChange('email_provider', option.value)}
                    >
                      <div class="p-icon">
                        {#if option.value === 'smtp'}
                          <Icon name="mail" size={24} />
                        {:else}
                          <Icon name="zap" size={24} />
                        {/if}
                      </div>
                      <div class="p-info">
                        <span class="p-name">{option.label}</span>
                        <span class="p-desc">
                          {#if option.value === 'smtp'}
                            Direct SMTP server connection.
                          {:else}
                            High-performance API delivery.
                          {/if}
                        </span>
                      </div>
                      <div class="p-check">
                        <Icon
                          name={localSettings['email_provider'] === option.value
                            ? 'check-circle'
                            : 'circle'}
                          size={20}
                        />
                      </div>
                    </button>
                  {/each}
                </div>

                <div class="config-panel fade-in">
                  <h3>
                    {$t('admin.settings.sections.sender_info') || 'Sender Information'}
                  </h3>
                  <div class="config-grid mb-6">
                    <div class="setting-item">
                      <label for="email-from-name"
                        >{$t('admin.settings.keys.email_from_name') || 'From Name'}</label
                      >
                      <Input
                        id="email-from-name"
                        value={localSettings['email_from_name']}
                        oninput={(e: any) => handleChange('email_from_name', e.target.value)}
                        placeholder={$t('admin.settings.email.placeholders.from_name') ||
                          'e.g. Acme Support'}
                      />
                    </div>
                    <div class="setting-item">
                      <label for="email-from-address"
                        >{$t('admin.settings.keys.email_from_address') || 'From Address'}</label
                      >
                      <Input
                        id="email-from-address"
                        value={localSettings['email_from_address']}
                        oninput={(e: any) => handleChange('email_from_address', e.target.value)}
                        placeholder={$t('admin.settings.email.placeholders.from_address') ||
                          'noreply@yourdomain.com'}
                      />
                    </div>
                  </div>

                  <div class="divider-line"></div>

                  <h3 class="mt-6">
                    {$t('admin.settings.email.connection_details') || 'Connection Details'}
                  </h3>
                  <div class="config-grid">
                    {#if localSettings['email_provider'] === 'smtp'}
                      <div class="setting-item">
                        <label for="smtp-host"
                          >{$t('admin.settings.keys.email_smtp_host') || 'SMTP Host'}</label
                        >
                        <Input
                          id="smtp-host"
                          value={localSettings['email_smtp_host']}
                          oninput={(e: any) => handleChange('email_smtp_host', e.target.value)}
                          placeholder={$t('admin.settings.email.placeholders.smtp_host') ||
                            'smtp.mailtrap.io'}
                        />
                      </div>
                      <div class="setting-item">
                        <label for="smtp-port"
                          >{$t('admin.settings.keys.email_smtp_port') || 'SMTP Port'}</label
                        >
                        <Input
                          id="smtp-port"
                          type="number"
                          value={localSettings['email_smtp_port']}
                          oninput={(e: any) => handleChange('email_smtp_port', e.target.value)}
                          placeholder={$t('admin.settings.email.placeholders.smtp_port') || '587'}
                        />
                      </div>
                      <div class="setting-item">
                        <label for="smtp-encryption"
                          >{$t('admin.settings.keys.email_smtp_encryption') || 'Encryption'}</label
                        >
                        <Select
                          id="smtp-encryption"
                          options={smtpEncryptionOptions}
                          value={localSettings['email_smtp_encryption']}
                          onchange={(e: any) => handleChange('email_smtp_encryption', e.detail)}
                        />
                      </div>
                      <div class="setting-item">
                        <label for="smtp-username"
                          >{$t('admin.settings.keys.email_smtp_username') || 'Username'}</label
                        >
                        <Input
                          id="smtp-username"
                          value={localSettings['email_smtp_username']}
                          oninput={(e: any) => handleChange('email_smtp_username', e.target.value)}
                        />
                      </div>
                      <div class="setting-item full-width">
                        <label for="smtp-password"
                          >{$t('admin.settings.keys.email_smtp_password') || 'Password'}</label
                        >
                        <Input
                          id="smtp-password"
                          type="password"
                          value={localSettings['email_smtp_password']}
                          oninput={(e: any) => handleChange('email_smtp_password', e.target.value)}
                          placeholder="••••••••••••"
                          showPasswordToggle={true}
                        />
                      </div>
                    {:else}
                      <div class="setting-item full-width">
                        <label for="api-key"
                          >{$t('admin.settings.keys.email_api_key') || 'API Key'}</label
                        >
                        <Input
                          id="api-key"
                          type="password"
                          value={localSettings['email_api_key']}
                          oninput={(e: any) => handleChange('email_api_key', e.target.value)}
                          placeholder="re_123456789..."
                          showPasswordToggle={true}
                        />
                      </div>
                    {/if}
                  </div>
                </div>

                <div class="config-panel fade-in mt-6">
                  <h3>
                    {$t('admin.settings.email.queue.title') || 'Delivery Queue & Retry'}
                  </h3>
                  <p class="muted">
                    {$t('admin.settings.email.queue.desc') ||
                      'Queue outgoing emails and automatically retry transient failures.'}
                  </p>
                  <div class="config-grid">
                    <div class="setting-item full-width">
                      <div class="toggle-row">
                        <div class="toggle-text">
                          <div class="toggle-title">
                            {$t('admin.settings.email.queue.enabled') || 'Enable Email Outbox'}
                          </div>
                          <div class="toggle-sub">
                            {$t('admin.settings.email.queue.enabled_desc') ||
                              'Recommended for production to prevent lost emails.'}
                          </div>
                        </div>
                        <label class="toggle">
                          <input
                            type="checkbox"
                            checked={localSettings['email_outbox_enabled'] === 'true'}
                            onchange={(e) =>
                              handleChange('email_outbox_enabled', e.currentTarget.checked)}
                          />
                          <span class="slider"></span>
                        </label>
                      </div>
                    </div>

                    <div class="setting-item">
                      <label for="email-outbox-max">
                        {$t('admin.settings.email.queue.max_attempts') || 'Max Attempts'}
                      </label>
                      <Input
                        id="email-outbox-max"
                        type="number"
                        value={localSettings['email_outbox_max_attempts']}
                        oninput={(e: any) =>
                          handleChange('email_outbox_max_attempts', e.target.value)}
                        placeholder="5"
                      />
                    </div>

                    <div class="setting-item">
                      <label for="email-outbox-delay">
                        {$t('admin.settings.email.queue.base_delay') || 'Base Delay (seconds)'}
                      </label>
                      <Input
                        id="email-outbox-delay"
                        type="number"
                        value={localSettings['email_outbox_base_delay_seconds']}
                        oninput={(e: any) =>
                          handleChange('email_outbox_base_delay_seconds', e.target.value)}
                        placeholder="30"
                      />
                    </div>
                  </div>

                  {#if $can('read', 'email_outbox')}
                    <div class="queue-actions">
                      <button
                        class="btn btn-secondary"
                        type="button"
                        onclick={() => goto('../email-outbox')}
                      >
                        <Icon name="mail" size={16} />
                        {$t('admin.settings.email.queue.view_outbox') || 'View Outbox'}
                      </button>
                    </div>
                  {/if}
                </div>

                <div class="test-email-card mt-6">
                  <div class="test-header">
                    <Icon name="send" size={18} />
                    <h4>
                      {$t('admin.settings.sections.test_configuration') || 'Test Configuration'}
                    </h4>
                  </div>
                  <p>
                    {$t('admin.settings.email.test.desc') ||
                      'Send a test email or verify SMTP connectivity.'}
                  </p>
                  <div class="test-form">
                    <Input
                      type="email"
                      value={testEmailAddress}
                      oninput={(e: any) => (testEmailAddress = e.target.value)}
                      placeholder={$t('admin.settings.email.test.recipient_placeholder') ||
                        'Enter recipient email'}
                    />
                    <div class="test-actions">
                      <button
                        class="btn btn-secondary"
                        onclick={sendTestEmail}
                        disabled={sendingTestEmail || !testEmailAddress}
                      >
                        {sendingTestEmail
                          ? $t('admin.settings.email.test.sending') || 'Sending...'
                          : $t('admin.settings.email.test.send') || 'Send Test'}
                      </button>

                      <button
                        class="btn btn-secondary"
                        onclick={testSmtpConnection}
                        disabled={testingSmtp}
                        title={$t('admin.settings.email.smtp_test.hint') ||
                          'Checks connectivity and auth without sending an email.'}
                      >
                        <Icon name="activity" size={16} />
                        {testingSmtp
                          ? $t('admin.settings.email.smtp_test.testing') || 'Testing...'
                          : $t('admin.settings.email.smtp_test.button') || 'Test SMTP'}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            {:else if activeTab === 'payment'}
              <!-- Payment Settings -->
              <div class="payment-settings">
                <span class="section-label"
                  >{$t('admin.settings.payment.methods_label') || 'Payment Methods'}</span
                >

                <!-- Midtrans Card -->
                <div class="method-card">
                  <div class="method-header">
                    <div class="m-icon midtrans">M</div>
                    <div class="m-info">
                      <h4>
                        {$t('admin.settings.sections.midtrans') || 'Midtrans Payment Gateway'}
                      </h4>
                      <p>Accept payments via Credit Card, GoPay, ShopeePay, VA, etc.</p>
                    </div>
                    <label class="toggle">
                      <input
                        type="checkbox"
                        checked={localSettings['payment_midtrans_enabled'] === 'true'}
                        onchange={(e) =>
                          handleChange('payment_midtrans_enabled', e.currentTarget.checked)}
                      />
                      <span class="slider"></span>
                    </label>
                  </div>

                  {#if localSettings['payment_midtrans_enabled'] === 'true'}
                    <div class="method-config fade-in">
                      <div class="config-grid">
                        <div class="setting-item">
                          <label for="midtrans-merchant-id"
                            >{$t('admin.settings.payment.midtrans.merchant_id') ||
                              'Merchant ID'}</label
                          >
                          <Input
                            id="midtrans-merchant-id"
                            value={localSettings['payment_midtrans_merchant_id']}
                            oninput={(e: any) =>
                              handleChange('payment_midtrans_merchant_id', e.target.value)}
                            placeholder="G123456789"
                          />
                        </div>
                        <div class="setting-item">
                          <label for="midtrans-client-key"
                            >{$t('admin.settings.payment.midtrans.client_key') ||
                              'Client Key'}</label
                          >
                          <Input
                            id="midtrans-client-key"
                            value={localSettings['payment_midtrans_client_key']}
                            oninput={(e: any) =>
                              handleChange('payment_midtrans_client_key', e.target.value)}
                            placeholder="SB-Mid-client-..."
                          />
                        </div>
                        <div class="setting-item full-width">
                          <label for="midtrans-server-key"
                            >{$t('admin.settings.payment.midtrans.server_key') ||
                              'Server Key'}</label
                          >
                          <Input
                            id="midtrans-server-key"
                            type="password"
                            value={localSettings['payment_midtrans_server_key']}
                            oninput={(e: any) =>
                              handleChange('payment_midtrans_server_key', e.target.value)}
                            placeholder="SB-Mid-server-..."
                            showPasswordToggle={true}
                          />
                        </div>
                        <div class="setting-item full-width checkbox-row">
                          <label class="checkbox-label">
                            <input
                              type="checkbox"
                              checked={localSettings['payment_midtrans_is_production'] === 'true'}
                              onchange={(e: any) =>
                                handleChange(
                                  'payment_midtrans_is_production',
                                  e.currentTarget.checked,
                                )}
                            />
                            <span>Enable Production Mode (Live)</span>
                          </label>
                        </div>
                      </div>
                    </div>
                  {/if}
                </div>

                <!-- Manual Transfer Card -->
                <div class="method-card mt-6">
                  <div class="method-header">
                    <div class="m-icon manual">
                      <Icon name="landmark" size={24} />
                    </div>
                    <div class="m-info">
                      <h4>
                        {$t('admin.settings.sections.bank_transfer_manual') ||
                          'Bank Transfer (Manual)'}
                      </h4>
                      <p>Accept payments via direct bank transfer verification.</p>
                    </div>
                    <label class="toggle">
                      <input
                        type="checkbox"
                        checked={localSettings['payment_manual_enabled'] === 'true'}
                        onchange={(e) =>
                          handleChange('payment_manual_enabled', e.currentTarget.checked)}
                      />
                      <span class="slider"></span>
                    </label>
                  </div>

                  {#if localSettings['payment_manual_enabled'] === 'true'}
                    <div class="method-config fade-in">
                      <div class="setting-item full-width">
                        <label for="payment-manual-instructions"
                          >{$t('admin.settings.payment.manual.instructions_label') ||
                            'Payment Instructions'}</label
                        >
                        <textarea
                          id="payment-manual-instructions"
                          class="form-textarea"
                          rows="4"
                          value={localSettings['payment_manual_instructions']}
                          oninput={(e: any) =>
                            handleChange('payment_manual_instructions', e.target.value)}
                          placeholder={$t(
                            'admin.settings.payment.manual.placeholder_instructions',
                          ) || 'Please transfer to BCA 1234567890 a/n PT Company...'}
                        ></textarea>
                        <p class="help-text">
                          These instructions will be shown to the user during checkout.
                        </p>
                      </div>
                      <div class="bank-accounts-manager mt-6">
                        <div class="bm-header">
                          <span class="label-text"
                            >{$t('admin.settings.payment.manual.bank_accounts') ||
                              'Bank Accounts'}</span
                          >
                          <button
                            class="btn btn-primary btn-sm"
                            onclick={() => (showAddBank = !showAddBank)}
                          >
                            <Icon name={showAddBank ? 'minus' : 'plus'} size={14} />
                            {showAddBank
                              ? $t('common.cancel') || 'Cancel'
                              : $t('admin.settings.payment.manual.add_bank') || 'Add Bank'}
                          </button>
                        </div>

                        {#if showAddBank}
                          <div class="add-bank-form fade-in">
                            <div class="form-row">
                              <Input
                                aria-label={$t(
                                  'admin.settings.payment.manual.bank_form.bank_name_label',
                                ) || 'Bank Name'}
                                value={newBank.bank_name}
                                oninput={(e: any) => (newBank.bank_name = e.target.value)}
                                placeholder={$t(
                                  'admin.settings.payment.manual.bank_form.bank_name_placeholder',
                                ) || 'Bank Name (e.g. BCA)'}
                              />
                              <Input
                                aria-label={$t(
                                  'admin.settings.payment.manual.bank_form.account_number_label',
                                ) || 'Account Number'}
                                value={newBank.account_number}
                                oninput={(e: any) => (newBank.account_number = e.target.value)}
                                placeholder={$t(
                                  'admin.settings.payment.manual.bank_form.account_number_placeholder',
                                ) || 'Account Number'}
                              />
                            </div>
                            <div class="form-row">
                              <Input
                                aria-label={$t(
                                  'admin.settings.payment.manual.bank_form.account_holder_label',
                                ) || 'Account Holder Name'}
                                value={newBank.account_holder}
                                oninput={(e: any) => (newBank.account_holder = e.target.value)}
                                placeholder={$t(
                                  'admin.settings.payment.manual.bank_form.account_holder_placeholder',
                                ) || 'Account Holder Name'}
                              />
                              <button class="btn btn-secondary" onclick={addBankAccount}
                                >{$t('admin.settings.payment.manual.bank_form.add') ||
                                  'Add'}</button
                              >
                            </div>
                          </div>
                        {/if}

                        <div class="bank-list-grid">
                          {#if bankAccounts.length === 0}
                            <div class="empty-state">
                              <div class="icon-placeholder">
                                <Icon name="landmark" size={24} />
                              </div>
                              <p>No bank accounts added yet.</p>
                              <button
                                class="btn btn-primary btn-sm mt-2"
                                onclick={() => (showAddBank = true)}
                                >{$t('admin.settings.payment.manual.add_one') || 'Add One'}</button
                              >
                            </div>
                          {:else}
                            {#each bankAccounts as bank}
                              <div class="bank-card-item">
                                <div class="bc-icon">
                                  <Icon name="landmark" size={20} />
                                </div>
                                <div class="bc-details">
                                  <span class="bc-name">{bank.bank_name}</span>
                                  <span class="bc-number">{bank.account_number}</span>
                                  <span class="bc-holder">{bank.account_holder}</span>
                                </div>
                                <div class="bc-actions">
                                  <button
                                    class="btn-icon delete"
                                    onclick={() => removeBankAccount(bank.id)}
                                    title={$t(
                                      'admin.settings.payment.manual.bank_form.remove_account',
                                    ) || 'Remove Account'}
                                  >
                                    <Icon name="trash" size={16} />
                                  </button>
                                </div>
                              </div>
                            {/each}
                            <button class="add-bank-card" onclick={() => (showAddBank = true)}>
                              <Icon name="plus" size={24} />
                              <span
                                >{$t('admin.settings.payment.manual.bank_form.add_account') ||
                                  'Add Account'}</span
                              >
                            </button>
                          {/if}
                        </div>
                      </div>
                    </div>
                  {/if}
                </div>
              </div>
            {:else}
              <div class="settings-list">
                {#each categories[activeTab as keyof typeof categories].keys as key}
                  <div class="setting-item">
                    <div class="setting-info">
                      <label for={key}>{getLabel(key)}</label>
                    </div>
                    <div class="setting-control">
                      {#if key === 'app_logo_path'}
                        <div class="file-upload">
                          {#if logoBase64}
                            <img src={logoBase64} class="logo-preview" alt="Logo" />
                          {/if}
                          <input type="file" accept="image/*" onchange={handleFileUpload} />
                        </div>
                      {:else if key.includes('password') || key.includes('secret') || key.includes('key')}
                        <Input
                          id={key}
                          type="password"
                          value={localSettings[key]}
                          oninput={(e: any) => handleChange(key, e.target.value)}
                        />
                      {:else if key === 'default_locale'}
                        <Select
                          id={key}
                          options={localeOptions}
                          value={localSettings[key]}
                          onchange={(e: any) => handleChange(key, e.detail)}
                        />
                      {:else if key === 'currency_code'}
                        <Select
                          id={key}
                          options={currencyCodeOptions}
                          value={localSettings[key]}
                          onchange={(e: any) => handleChange(key, e.detail)}
                        />
                      {:else if key === 'storage_driver'}
                        <Select
                          id={key}
                          options={storageOptions}
                          value={localSettings[key]}
                          onchange={(e: any) => handleChange(key, e.detail)}
                        />
                      {:else if key === 'email_provider'}
                        <Select
                          id={key}
                          options={emailProviderOptions}
                          value={localSettings[key]}
                          onchange={(e: any) => handleChange(key, e.detail)}
                        />
                      {:else if key === 'email_smtp_encryption'}
                        <Select
                          id={key}
                          options={smtpEncryptionOptions}
                          value={localSettings[key]}
                          onchange={(e: any) => handleChange(key, e.detail)}
                        />
                      {:else}
                        <Input
                          id={key}
                          value={localSettings[key]}
                          oninput={(e: any) => handleChange(key, e.target.value)}
                        />
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <div class="card-footer">
            <button
              class="btn btn-secondary"
              disabled={!hasChanges || saving}
              onclick={discardChanges}>{$t('common.reset') || 'Reset'}</button
            >
            <button class="btn btn-primary" disabled={!hasChanges || saving} onclick={saveChanges}>
              {saving
                ? $t('common.saving') || 'Saving...'
                : $t('common.save_changes') || 'Save Changes'}
            </button>
          </div>
        </div>
      {/if}
    </main>
  </div>

  <MobileFabMenu
    items={mobileMenuItems}
    {activeTab}
    title={$t('topbar.titles.settings') || 'Settings'}
    on:change={(e) => {
      activeTab = e.detail;
      // Keep unsaved edits when switching tabs (avoid refetch/reset).
    }}
  />
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 1.5rem);
    max-width: 1400px;
    margin: 0 auto;
    --glass: rgba(255, 255, 255, 0.04);
    --glass-2: rgba(255, 255, 255, 0.02);
    --glass-border: rgba(255, 255, 255, 0.08);
    --code-bg: rgba(255, 255, 255, 0.06);
  }

  :global([data-theme='light']) .page-container {
    --glass: rgba(0, 0, 0, 0.02);
    --glass-2: rgba(0, 0, 0, 0.015);
    --glass-border: rgba(0, 0, 0, 0.06);
    --code-bg: rgba(0, 0, 0, 0.05);
  }
  .layout-grid {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 1.5rem;
    align-items: start;
  }

  .sidebar {
    background: linear-gradient(145deg, var(--glass), var(--glass-2));
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    padding: 0.75rem;
    position: sticky;
    top: 1.5rem;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
    backdrop-filter: blur(10px);
  }

  :global([data-theme='light']) .sidebar {
    background: linear-gradient(135deg, #ffffff, #f7f7fb);
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.08),
      0 0 0 1px rgba(255, 255, 255, 0.8);
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.7rem 0.9rem;
    width: 100%;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    font-weight: 600;
    cursor: pointer;
    border-radius: 12px;
    text-align: left;
    transition: all 0.2s;
  }
  .nav-item:hover {
    background: rgba(99, 102, 241, 0.08);
    color: var(--text-primary);
  }
  .nav-item.active {
    background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.18), transparent 60%);
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  :global([data-theme='light']) .nav-item.active {
    background: radial-gradient(circle at 20% 20%, rgba(99, 102, 241, 0.12), transparent 60%);
    border-color: rgba(99, 102, 241, 0.25);
  }

  .card {
    background: linear-gradient(145deg, var(--glass), var(--glass-2));
    border: 1px solid var(--glass-border);
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
    backdrop-filter: blur(10px);
  }

  :global([data-theme='light']) .card {
    background: linear-gradient(135deg, #ffffff, #f7f7fb);
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.08),
      0 0 0 1px rgba(255, 255, 255, 0.8);
  }
  .card-header {
    padding: 1.25rem 1.75rem;
    border-bottom: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.015);
  }
  .card-title {
    font-size: 1.25rem;
    font-weight: 800;
    margin: 0;
    letter-spacing: 0.01em;
  }
  .card-subtitle {
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin: 0.25rem 0 0;
  }

  .settings-body {
    padding: 1.5rem 1.75rem;
  }
  .settings-list {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
  }
  .setting-item {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .settings-list .setting-item {
    padding: 1rem;
    border-radius: 16px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
    transition: border-color 0.2s ease;
  }

  :global([data-theme='light']) .settings-list .setting-item {
    background: rgba(255, 255, 255, 0.7);
  }

  .settings-list .setting-item:hover {
    border-color: rgba(99, 102, 241, 0.25);
  }

  .setting-item-row {
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 1rem;
    border-radius: 16px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
  }

  :global([data-theme='light']) .setting-item-row {
    background: rgba(255, 255, 255, 0.7);
  }
  .setting-info label {
    font-weight: 650;
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .setting-info h3 {
    font-size: 1rem;
    font-weight: 800;
    margin: 0;
    color: var(--text-primary);
    letter-spacing: 0.01em;
  }

  .setting-info p {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.4;
  }

  .setting-group {
    margin-bottom: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .setting-group label {
    font-weight: 650;
    color: var(--text-primary);
    font-size: 0.9rem;
  }
  .help-text {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }
  code {
    background: var(--code-bg);
    border: 1px solid var(--glass-border);
    padding: 0.12rem 0.35rem;
    border-radius: 8px;
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  .file-upload {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .file-upload input[type='file'] {
    width: 100%;
    max-width: 320px;
  }

  .upgrade-banner {
    background: linear-gradient(135deg, rgba(99, 102, 241, 0.1), rgba(236, 72, 153, 0.1));
    border: 1px solid var(--color-primary-subtle);
    border-radius: 16px;
    padding: 1rem;
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .icon-box {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
    flex-shrink: 0;
  }
  .upgrade-banner .text {
    flex: 1;
  }
  .upgrade-banner h4 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 750;
    color: var(--text-primary);
  }
  .upgrade-banner p {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .card-footer {
    padding: 1.25rem 1.5rem;
    background: rgba(255, 255, 255, 0.015);
    border-top: 1px solid var(--glass-border);
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .logo-preview {
    width: 44px;
    height: 44px;
    object-fit: contain;
    border-radius: 12px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
  }
  .loading-state {
    padding: 4rem;
    display: flex;
    justify-content: center;
  }
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.12);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  :global([data-theme='light']) .spinner {
    border-color: rgba(0, 0, 0, 0.12);
    border-top-color: var(--color-primary);
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 900px) {
    .layout-grid {
      grid-template-columns: 1fr;
    }
    .desktop-sidebar {
      display: none;
    }
    .settings-body {
      padding: 1.25rem;
    }
    .card-header {
      padding: 1.1rem 1.25rem;
    }
    .settings-list {
      grid-template-columns: 1fr;
    }
  }

  /* Toggle Switch */
  .toggle {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .toggle-text {
    min-width: 0;
  }

  .toggle-title {
    font-weight: 800;
    color: var(--text-primary);
  }

  .toggle-sub {
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.85rem;
    line-height: 1.35;
    font-weight: 600;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(255, 255, 255, 0.1);
    transition: 0.3s;
    border-radius: 24px;
    border: 1px solid var(--glass-border);
  }

  :global([data-theme='light']) .slider {
    background-color: rgba(0, 0, 0, 0.06);
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: 0.3s;
    border-radius: 50%;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  input:checked + .slider {
    background-color: var(--color-primary);
    border-color: rgba(99, 102, 241, 0.4);
  }

  input:checked + .slider:before {
    transform: translateX(20px);
  }

  /* Storage UI */
  .section-label {
    font-weight: 750;
    color: var(--text-primary);
    margin-bottom: 0.9rem;
    display: block;
    font-size: 0.95rem;
  }
  .provider-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1rem;
    margin-bottom: 1.75rem;
  }

  .provider-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.1rem 1.15rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
  }
  .provider-card:hover {
    border-color: rgba(99, 102, 241, 0.28);
    background: rgba(99, 102, 241, 0.06);
    transform: translateY(-1px);
  }
  .provider-card.selected {
    border-color: rgba(99, 102, 241, 0.42);
    background: radial-gradient(
      circle at 20% 20%,
      rgba(99, 102, 241, 0.18),
      rgba(255, 255, 255, 0.03)
    );
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.22);
  }

  :global([data-theme='light']) .provider-card {
    background: rgba(255, 255, 255, 0.75);
  }

  :global([data-theme='light']) .provider-card:hover {
    background: rgba(99, 102, 241, 0.06);
  }

  :global([data-theme='light']) .provider-card.selected {
    background: radial-gradient(
      circle at 20% 20%,
      rgba(99, 102, 241, 0.12),
      rgba(255, 255, 255, 0.75)
    );
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.08);
  }

  .p-icon {
    width: 42px;
    height: 42px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid var(--glass-border);
  }
  .selected .p-icon {
    background: rgba(99, 102, 241, 0.16);
    color: var(--text-primary);
    border-color: rgba(99, 102, 241, 0.3);
  }

  .p-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .p-name {
    font-weight: 750;
    color: var(--text-primary);
    font-size: 0.95rem;
    line-height: 1.2;
  }
  .p-desc {
    font-size: 0.82rem;
    color: var(--text-secondary);
    margin-top: 0.15rem;
  }

  .p-check {
    color: rgba(255, 255, 255, 0.18);
  }

  :global([data-theme='light']) .p-check {
    color: rgba(0, 0, 0, 0.18);
  }

  .selected .p-check {
    color: rgba(99, 102, 241, 0.9);
  }

  .config-panel {
    background: rgba(255, 255, 255, 0.02);
    padding: 1.25rem;
    border-radius: 16px;
    border: 1px solid var(--glass-border);
  }

  :global([data-theme='light']) .config-panel {
    background: rgba(255, 255, 255, 0.75);
  }

  .config-panel h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    font-weight: 800;
    letter-spacing: 0.01em;
  }
  .config-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
  }

  .divider-line {
    height: 1px;
    background: var(--glass-border);
    margin: 1.5rem 0;
  }
  .mb-6 {
    margin-bottom: 1.5rem;
  }
  .mt-6 {
    margin-top: 1.5rem;
  }
  .full-width {
    grid-column: 1 / -1;
  }

  /* Test Email UI */
  .test-email-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    padding: 1.1rem 1.25rem;
  }

  :global([data-theme='light']) .test-email-card {
    background: rgba(255, 255, 255, 0.75);
  }

  .test-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--text-primary);
    margin-bottom: 0.35rem;
    font-weight: 750;
  }
  .test-header h4 {
    margin: 0;
    font-size: 1rem;
    font-weight: 800;
  }
  .test-email-card p {
    font-size: 0.88rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }
  .test-form {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    align-items: center;
  }
  .test-form :global(.input-wrapper) {
    flex: 1;
    min-width: 220px;
  }
  .test-actions {
    display: inline-flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    align-items: center;
  }

  @media (max-width: 640px) {
    .config-grid {
      grid-template-columns: 1fr;
    }
    .test-form {
      flex-direction: column;
      align-items: stretch;
    }
    .test-form :global(.input-wrapper) {
      min-width: unset;
    }
    .form-row {
      grid-template-columns: 1fr;
    }
    .setting-item-row {
      flex-direction: column;
      align-items: stretch;
    }
    .setting-item-row .toggle {
      align-self: flex-end;
    }
  }

  /* Payment UI */
  .method-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    overflow: hidden;
  }

  :global([data-theme='light']) .method-card {
    background: rgba(255, 255, 255, 0.75);
  }

  .method-header {
    padding: 1.1rem 1.25rem;
    display: flex;
    align-items: center;
    gap: 1rem;
    border-bottom: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.015);
  }
  .m-icon {
    width: 42px;
    height: 42px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }
  .m-icon.midtrans {
    background: linear-gradient(135deg, #002c5f, #0b1b39);
    border-color: rgba(0, 44, 95, 0.45);
    color: white;
  }
  .m-icon.manual {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
  }

  .m-info {
    flex: 1;
    min-width: 180px;
  }
  .m-info h4 {
    margin: 0;
    font-size: 1rem;
    color: var(--text-primary);
    font-weight: 800;
  }
  .m-info p {
    margin: 0.25rem 0 0;
    font-size: 0.88rem;
    color: var(--text-secondary);
  }

  .method-config {
    padding: 1.25rem;
    background: rgba(255, 255, 255, 0.02);
  }

  :global([data-theme='light']) .method-config {
    background: rgba(0, 0, 0, 0.015);
  }
  .checkbox-row {
    display: flex;
    align-items: center;
    margin-top: 0.5rem;
  }
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
    font-size: 0.9rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .form-textarea {
    width: 100%;
    padding: 0.75rem 0.9rem;
    border-radius: 14px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.92rem;
    resize: vertical;
    transition:
      border-color 0.2s,
      box-shadow 0.2s;
  }

  :global([data-theme='light']) .form-textarea {
    background: rgba(255, 255, 255, 0.75);
  }
  .form-textarea:focus {
    outline: none;
    border-color: rgba(99, 102, 241, 0.35);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.14);
  }

  /* Bank Manager UI */
  .bank-accounts-manager {
    margin-top: 1.5rem;
    border-top: 1px dashed var(--glass-border);
    padding-top: 1.25rem;
  }
  .bm-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .label-text {
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: 0.01em;
  }

  .add-bank-form {
    display: grid;
    gap: 0.75rem;
    padding: 1rem;
    border-radius: 16px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
    margin-bottom: 1rem;
  }

  :global([data-theme='light']) .add-bank-form {
    background: rgba(255, 255, 255, 0.75);
  }

  .form-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
    align-items: end;
  }

  .form-row :global(.btn) {
    width: 100%;
  }

  .mt-2 {
    margin-top: 0.5rem;
  }

  .bank-list-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 1rem;
  }

  .bank-card-item {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--glass-border);
    border-radius: 16px;
    padding: 1.1rem 1.15rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    position: relative;
    transition: all 0.2s;
  }

  :global([data-theme='light']) .bank-card-item {
    background: rgba(255, 255, 255, 0.75);
  }

  .bank-card-item:hover {
    border-color: rgba(99, 102, 241, 0.25);
    transform: translateY(-1px);
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.22);
  }

  :global([data-theme='light']) .bank-card-item:hover {
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.08);
  }

  .bc-icon {
    width: 38px;
    height: 38px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    border: 1px solid var(--glass-border);
  }
  .bc-details {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .bc-name {
    font-weight: 850;
    color: var(--text-primary);
    font-size: 0.98rem;
  }
  .bc-number {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
    font-size: 1rem;
    letter-spacing: 0.05em;
    color: var(--text-primary);
  }
  .bc-holder {
    font-size: 0.78rem;
    color: var(--text-secondary);
    text-transform: uppercase;
  }

  .bc-actions {
    position: absolute;
    top: 0.9rem;
    right: 0.9rem;
  }

  .btn-icon {
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 12px;
    border: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background: rgba(99, 102, 241, 0.1);
    color: var(--text-primary);
    border-color: rgba(99, 102, 241, 0.3);
  }

  .btn-icon.delete:hover {
    background: rgba(239, 68, 68, 0.12);
    color: #ef4444;
    border-color: rgba(239, 68, 68, 0.25);
  }

  .add-bank-card {
    border: 2px dashed var(--glass-border);
    background: rgba(255, 255, 255, 0.01);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    min-height: 150px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
  }
  .add-bank-card:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
    background: rgba(99, 102, 241, 0.06);
    transform: translateY(-1px);
  }
  .add-bank-card span {
    font-weight: 750;
    font-size: 0.92rem;
  }

  .empty-state {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem;
    background: rgba(255, 255, 255, 0.02);
    border-radius: 16px;
    border: 1px solid var(--glass-border);
    color: var(--text-secondary);
  }

  :global([data-theme='light']) .empty-state {
    background: rgba(255, 255, 255, 0.75);
  }
  .icon-placeholder {
    width: 48px;
    height: 48px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 1rem;
    color: var(--text-secondary);
    border: 1px solid var(--glass-border);
  }

  .queue-actions {
    margin-top: 0.9rem;
    display: flex;
    justify-content: flex-end;
  }
</style>

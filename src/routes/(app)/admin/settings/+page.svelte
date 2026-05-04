<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import type { Component } from 'svelte';
  import { api } from '$lib/api/client';
  import { user, tenant, isAdmin, can, getToken } from '$lib/stores/auth';
  import { appSettings } from '$lib/stores/settings';
  import { appLogo } from '$lib/stores/logo';
  import { goto } from '$app/navigation';
  import { locale, t, waitLocale } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MobileFabMenu from '$lib/components/ui/MobileFabMenu.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import type { EmailVerificationReadiness, Setting } from '$lib/api/client';
  import { toast } from 'svelte-sonner';
  import { get } from 'svelte/store';
  import { adminSettingsCache } from '$lib/stores/adminSettingsCache';
  import { getAdminBillingNavigation } from '$lib/utils/adminBillingNavigation';
  import {
    loadAdminSettingsEmailTab,
    loadAdminSettingsNotificationEventsTab,
    loadAdminSettingsPaymentTab,
    loadAdminSettingsWhatsAppTab,
    loadTenantBillingPlanPanel,
  } from './adminSettingsPageModules';
  import { WHATSAPP_GATEWAY_SETTING_KEYS } from '$lib/utils/whatsappGateway';

  type DeferredComponent = Component<any>;

  let loading = $state(true);
  let saving = $state(false);
  let emailVerificationReadiness = $state<EmailVerificationReadiness>({
    ready: true,
    reason: null,
  });
  let settings = $state<Record<string, Setting>>({});
  let localSettings = $state<Record<string, string>>({});
  let logoBase64 = $state<string | null>(null);
  let activeTab = $state('general');
  let hasChanges = $state(false);
  let isMobile = $state(false);
  let billingPlanPanelLoading = $state(false);
  let emailTabLoading = $state(false);
  let paymentTabLoading = $state(false);
  let whatsappTabLoading = $state(false);
  let notificationEventsTabLoading = $state(false);
  let TenantBillingPlanPanelComponent = $state<DeferredComponent | null>(null);
  let SettingsEmailTabComponent = $state<DeferredComponent | null>(null);
  let SettingsPaymentTabComponent = $state<DeferredComponent | null>(null);
  let SettingsWhatsAppTabComponent = $state<DeferredComponent | null>(null);
  let SettingsNotificationEventsTabComponent = $state<DeferredComponent | null>(null);

  // Tenant specific state
  let tenantInfo = $state<any>(null);
  let tenantChanges = $state<{ name?: string; customDomain?: string }>({});
  let customDomainAccess = $state(false);

  // Baseline snapshot for local reset (no network)
  let baselineLocalSettings = $state<Record<string, string>>({});
  let baselineLogoBase64 = $state<string | null>(null);
  let baselineTenantInfo = $state<any>(null);
  let baselineCustomDomainAccess = $state(false);
  const billingNav = $derived.by(() =>
    getAdminBillingNavigation({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantSubscriptionPath = $derived(billingNav.subscriptionPath);
  const billingPlanSettingsPath = $derived(billingNav.billingPlanSettingsPath);

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
    billing_plan: {
      label: $t('admin.settings.categories.billing_plan') || 'Billing & Plan',
      icon: 'credit-card',
      keys: [],
    },
    security: {
      label: $t('admin.settings.categories.security') || 'Security',
      icon: 'shield',
      keys: ['auth_require_email_verification', 'customer_self_registration_enabled'],
    },
    network: {
      label: $t('admin.settings.categories.network') || 'Network',
      icon: 'activity',
      keys: [
        'mikrotik_alerting_enabled',
        'mikrotik_alert_offline_after_secs',
        'mikrotik_alert_cpu_risk',
        'mikrotik_alert_cpu_hot',
        'mikrotik_alert_latency_risk_ms',
        'mikrotik_alert_latency_hot_ms',
        'mikrotik_incident_sla_warn_minutes',
        'mikrotik_incident_sla_breach_minutes',
        'mikrotik_incident_correlation_enabled',
        'mikrotik_incident_auto_escalation_enabled',
        'mikrotik_incident_escalation_minutes',
        'mikrotik_alert_email_enabled',
        'mikrotik_incident_assignment_email_enabled',
        'pppoe_auto_apply_on_save_enabled',
      ],
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
        'payment_duitku_enabled',
        'payment_duitku_merchant_code',
        'payment_duitku_api_key',
        'payment_duitku_payment_method',
        'payment_duitku_payment_methods',
        'payment_duitku_is_production',
        'payment_manual_enabled',
        'payment_manual_instructions',
        'payment_manual_accounts',
        'customer_invoice_auto_generate_enabled',
        'customer_invoice_generate_days_before_due',
        'customer_invoice_scheduler_interval_minutes',
        'customer_invoice_last_run_at',
      ],
    },
    whatsapp: {
      label: $t('admin.settings.categories.whatsapp') || 'WhatsApp Gateway',
      icon: 'message-circle',
      keys: [...WHATSAPP_GATEWAY_SETTING_KEYS],
    },
    event_notifications: {
      label: $t('admin.settings.categories.event_notifications') || 'Event Notifications',
      icon: 'bell',
      keys: ['wa_events_tenant'],
    },
  }));

  let mobileMenuItems = $derived(
    Object.entries(categories).map(([id, cat]) => ({
      id,
      label: cat.label,
      icon: cat.icon,
    })),
  );

  function isSettingsTab(tab: string | null): tab is keyof typeof categories {
    return Boolean(tab && Object.prototype.hasOwnProperty.call(categories, tab));
  }

  function resolveSettingsTabFromUrl() {
    if (typeof window === 'undefined') return null;

    const hashTab = decodeURIComponent(window.location.hash.replace(/^#/, '')).trim();
    if (isSettingsTab(hashTab)) return hashTab;

    const queryTab = new URLSearchParams(window.location.search).get('tab');
    if (isSettingsTab(queryTab)) return queryTab;

    return null;
  }

  function syncSettingsTabHash(tab: string, replace = false) {
    if (typeof window === 'undefined' || !isSettingsTab(tab)) return;

    const url = new URL(window.location.href);
    url.searchParams.delete('tab');
    url.hash = tab;

    if (replace) {
      window.history.replaceState(window.history.state, '', url);
    } else {
      window.history.pushState(window.history.state, '', url);
    }
  }

  function selectSettingsTab(tab: string, options: { discard?: boolean; replace?: boolean } = {}) {
    if (!isSettingsTab(tab)) return;

    activeTab = tab;
    syncSettingsTabHash(tab, options.replace);
    if (options.discard) discardChanges();
  }

  function applySettingsTabFromUrl() {
    activeTab = resolveSettingsTabFromUrl() || 'general';
  }

  onMount(async () => {
    if (!$isAdmin || !$can('read', 'settings')) {
      goto('/unauthorized');
      return;
    }

    if (typeof window !== 'undefined') {
      const tab = resolveSettingsTabFromUrl();
      if (tab) selectSettingsTab(tab, { replace: true });

      window.addEventListener('hashchange', applySettingsTabFromUrl);
      window.addEventListener('popstate', applySettingsTabFromUrl);
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

  async function ensureTenantBillingPlanPanelLoaded() {
    if (TenantBillingPlanPanelComponent || billingPlanPanelLoading) return;

    billingPlanPanelLoading = true;
    try {
      const { BillingPlanPanelComponent } = await loadTenantBillingPlanPanel();
      TenantBillingPlanPanelComponent = BillingPlanPanelComponent;
    } catch (error: any) {
      toast.error(error?.message || 'Failed to load billing plan panel');
    } finally {
      billingPlanPanelLoading = false;
    }
  }

  async function ensureEmailTabLoaded() {
    if (SettingsEmailTabComponent || emailTabLoading) return;

    emailTabLoading = true;
    try {
      const { SettingsEmailTabComponent: EmailTabComponent } = await loadAdminSettingsEmailTab();
      SettingsEmailTabComponent = EmailTabComponent;
    } catch (error: any) {
      toast.error(error?.message || 'Failed to load email settings tab');
    } finally {
      emailTabLoading = false;
    }
  }

  async function ensurePaymentTabLoaded() {
    if (SettingsPaymentTabComponent || paymentTabLoading) return;

    paymentTabLoading = true;
    try {
      const { SettingsPaymentTabComponent: PaymentTabComponent } =
        await loadAdminSettingsPaymentTab();
      SettingsPaymentTabComponent = PaymentTabComponent;
    } catch (error: any) {
      toast.error(error?.message || 'Failed to load payment settings tab');
    } finally {
      paymentTabLoading = false;
    }
  }

  async function ensureWhatsAppTabLoaded() {
    if (SettingsWhatsAppTabComponent || whatsappTabLoading) return;

    whatsappTabLoading = true;
    try {
      const { SettingsWhatsAppTabComponent: WhatsAppTabComponent } =
        await loadAdminSettingsWhatsAppTab();
      SettingsWhatsAppTabComponent = WhatsAppTabComponent;
    } catch (error: any) {
      toast.error(error?.message || 'Failed to load WhatsApp settings tab');
    } finally {
      whatsappTabLoading = false;
    }
  }

  async function ensureNotificationEventsTabLoaded() {
    if (SettingsNotificationEventsTabComponent || notificationEventsTabLoading) return;

    notificationEventsTabLoading = true;
    try {
      const { SettingsNotificationEventsTabComponent: NotificationEventsTabComponent } =
        await loadAdminSettingsNotificationEventsTab();
      SettingsNotificationEventsTabComponent = NotificationEventsTabComponent;
    } catch (error: any) {
      toast.error(error?.message || 'Failed to load event notifications tab');
    } finally {
      notificationEventsTabLoading = false;
    }
  }

  let activeCategory = $derived(categories[activeTab as keyof typeof categories]);

  $effect(() => {
    if (activeTab !== 'billing_plan') return;
    void ensureTenantBillingPlanPanelLoaded();
  });
  $effect(() => {
    if (activeTab !== 'email') return;
    void ensureEmailTabLoaded();
  });
  $effect(() => {
    if (activeTab !== 'payment') return;
    void ensurePaymentTabLoaded();
  });
  $effect(() => {
    if (activeTab !== 'whatsapp') return;
    void ensureWhatsAppTabLoaded();
  });
  $effect(() => {
    if (activeTab !== 'event_notifications') return;
    void ensureNotificationEventsTabLoaded();
  });
  let slaWarnPreview = $derived.by(() => {
    const raw = localSettings['mikrotik_incident_sla_warn_minutes'] || '30';
    const parsed = Number.parseInt(String(raw), 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 30;
  });
  let slaBreachPreview = $derived.by(() => {
    const raw = localSettings['mikrotik_incident_sla_breach_minutes'] || '120';
    const parsed = Number.parseInt(String(raw), 10);
    const val = Number.isFinite(parsed) && parsed > 0 ? parsed : 120;
    return val > slaWarnPreview ? val : slaWarnPreview * 2;
  });

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
        if (key === 'mikrotik_alerting_enabled' && !val) val = 'true';
        if (key === 'mikrotik_alert_offline_after_secs' && !val) val = '60';
        if (key === 'mikrotik_alert_cpu_risk' && !val) val = '70';
        if (key === 'mikrotik_alert_cpu_hot' && !val) val = '85';
        if (key === 'mikrotik_alert_latency_risk_ms' && !val) val = '200';
        if (key === 'mikrotik_alert_latency_hot_ms' && !val) val = '400';
        if (key === 'mikrotik_incident_sla_warn_minutes' && !val) val = '30';
        if (key === 'mikrotik_incident_sla_breach_minutes' && !val) val = '120';
        if (key === 'mikrotik_incident_correlation_enabled' && !val) val = 'true';
        if (key === 'mikrotik_incident_auto_escalation_enabled' && !val) val = 'false';
        if (key === 'mikrotik_incident_escalation_minutes' && !val) val = '60';
        if (key === 'mikrotik_alert_email_enabled' && !val) val = 'false';
        if (key === 'mikrotik_incident_assignment_email_enabled' && !val) val = 'false';
        if (key === 'pppoe_auto_apply_on_save_enabled' && !val) val = 'false';
        if (key === 'auth_require_email_verification' && !val) val = 'false';
        if (key === 'customer_self_registration_enabled' && !val) val = 'false';
        if (key === 'wa_gateway_enabled' && !val) val = 'false';
        if (key === 'wa_gateway_provider' && !val) val = 'disabled';
        if (key === 'wa_gateway_custom_method' && !val) val = 'POST';
        if (key === 'wa_gateway_custom_success_statuses' && !val) val = '200,201,202';
        if (key === 'wa_events_tenant' && !val) val = '{}';
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

      const [_, data, tenant, readiness] = await Promise.all([
        appLogo.refresh(token).catch(() => null),
        api.settings.getAll(),
        api.tenant.getSelf(),
        api.settings
          .getEmailVerificationReadiness()
          .catch(() => ({ ready: true, reason: null }) as EmailVerificationReadiness),
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
      emailVerificationReadiness = readiness;

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
    if (
      key === 'auth_require_email_verification' &&
      Boolean(value) &&
      !emailVerificationReadiness.ready
    ) {
      toast.error(
        emailVerificationReadiness.reason ||
          get(t)('admin.settings.security.require_email_verification_not_ready') ||
          'Email configuration is not ready. Configure Email settings first.',
      );
      return;
    }

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

  function formatLastRunAt(value?: string) {
    if (!value) return '-';
    const dt = new Date(value);
    if (Number.isNaN(dt.getTime())) return value;
    return new Intl.DateTimeFormat($locale || undefined, {
      year: 'numeric',
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZone: $appSettings.app_timezone || 'UTC',
    }).format(dt);
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
    if (
      activeTab === 'security' &&
      localSettings['auth_require_email_verification'] === 'true' &&
      !emailVerificationReadiness.ready
    ) {
      toast.error(
        emailVerificationReadiness.reason ||
          get(t)('admin.settings.security.require_email_verification_not_ready') ||
          'Email configuration is not ready. Configure Email settings first.',
      );
      return;
    }

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
              selectSettingsTab(id, { discard: true });
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
                      onclick={() => goto(billingPlanSettingsPath)}
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
            {:else if activeTab === 'billing_plan'}
              {#if TenantBillingPlanPanelComponent}
                <TenantBillingPlanPanelComponent
                  openSubscription={() => goto(tenantSubscriptionPath)}
                />
              {:else}
                <div class="loading-state" aria-busy={billingPlanPanelLoading}>
                  <div class="spinner"></div>
                </div>
              {/if}
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

              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>
                    {$t('admin.settings.security.require_email_verification_title') ||
                      'Require Email Verification'}
                  </h3>
                  <p>
                    {$t('admin.settings.security.require_email_verification_desc') ||
                      'Require users in this tenant to verify email before login. Can only be enabled after email provider is configured.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['auth_require_email_verification'] === 'true'}
                    disabled={!emailVerificationReadiness.ready &&
                      localSettings['auth_require_email_verification'] !== 'true'}
                    onchange={(e) =>
                      handleChange('auth_require_email_verification', e.currentTarget.checked)}
                  />
                  <span class="slider"></span>
                </label>
              </div>
              {#if !emailVerificationReadiness.ready}
                <p class="help-text warning-text">
                  {emailVerificationReadiness.reason ||
                    $t('admin.settings.security.require_email_verification_not_ready') ||
                    'Email configuration is not ready. Configure Email settings first.'}
                </p>
              {/if}

              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>
                    {$t('admin.settings.security.customer_self_registration_title') ||
                      'Customer Self Registration'}
                  </h3>
                  <p>
                    {$t('admin.settings.security.customer_self_registration_desc') ||
                      'Allow customer signup from this tenant custom domain. Default is disabled.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['customer_self_registration_enabled'] === 'true'}
                    onchange={(e) =>
                      handleChange('customer_self_registration_enabled', e.currentTarget.checked)}
                  />
                  <span class="slider"></span>
                </label>
              </div>
            {:else if activeTab === 'network'}
              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>{$t('admin.settings.network.alerting.title') || 'Router Alerting'}</h3>
                  <p>
                    {$t('admin.settings.network.alerting.desc') ||
                      'Enable incidents and notifications derived from MikroTik polling.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['mikrotik_alerting_enabled'] === 'true'}
                    onchange={(e) =>
                      handleChange('mikrotik_alerting_enabled', e.currentTarget.checked)}
                  />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="config-panel fade-in mt-6">
                <h3>{$t('admin.settings.network.thresholds.title') || 'Thresholds'}</h3>
                <p class="help-text">
                  {$t('admin.settings.network.thresholds.desc') ||
                    'Used by NOC filters and by the background poller to open incidents.'}
                </p>
                <p class="help-text">
                  {$t('admin.settings.network.thresholds.sla_preview', {
                    values: { warn: slaWarnPreview, breach: slaBreachPreview },
                  }) ||
                    `Incidents become warning after ${slaWarnPreview} minutes and breach after ${slaBreachPreview} minutes.`}
                </p>

                <div class="config-grid">
                  <div class="setting-item">
                    <label for="mikrotik-alert-offline-after">
                      {$t('admin.settings.network.thresholds.offline_after_secs') ||
                        'Offline incident after (seconds)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-alert-offline-after"
                        type="number"
                        min="0"
                        value={localSettings['mikrotik_alert_offline_after_secs']}
                        oninput={(e: any) =>
                          handleChange('mikrotik_alert_offline_after_secs', e.target.value)}
                      />
                    </div>
                  </div>

                  <div class="setting-item">
                    <label for="mikrotik-alert-cpu-risk">
                      {$t('admin.settings.network.thresholds.cpu_risk') || 'CPU risk (%)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-alert-cpu-risk"
                        type="number"
                        value={localSettings['mikrotik_alert_cpu_risk']}
                        oninput={(e: any) =>
                          handleChange('mikrotik_alert_cpu_risk', e.target.value)}
                      />
                    </div>
                  </div>

                  <div class="setting-item">
                    <label for="mikrotik-alert-cpu-hot">
                      {$t('admin.settings.network.thresholds.cpu_hot') || 'CPU hot (%)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-alert-cpu-hot"
                        type="number"
                        value={localSettings['mikrotik_alert_cpu_hot']}
                        oninput={(e: any) => handleChange('mikrotik_alert_cpu_hot', e.target.value)}
                      />
                    </div>
                  </div>

                  <div class="setting-item">
                    <label for="mikrotik-alert-lat-risk">
                      {$t('admin.settings.network.thresholds.latency_risk') || 'Latency risk (ms)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-alert-lat-risk"
                        type="number"
                        value={localSettings['mikrotik_alert_latency_risk_ms']}
                        oninput={(e: any) =>
                          handleChange('mikrotik_alert_latency_risk_ms', e.target.value)}
                      />
                    </div>
                  </div>

                  <div class="setting-item">
                    <label for="mikrotik-alert-lat-hot">
                      {$t('admin.settings.network.thresholds.latency_hot') || 'Latency hot (ms)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-alert-lat-hot"
                        type="number"
                        value={localSettings['mikrotik_alert_latency_hot_ms']}
                        oninput={(e: any) =>
                          handleChange('mikrotik_alert_latency_hot_ms', e.target.value)}
                      />
                    </div>
                  </div>

                  <div class="setting-item">
                    <label for="mikrotik-sla-warn-minutes">
                      {$t('admin.settings.network.thresholds.incident_sla_warn_minutes') ||
                        'Incident SLA warning (minutes)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-sla-warn-minutes"
                        type="number"
                        min="1"
                        value={localSettings['mikrotik_incident_sla_warn_minutes']}
                        oninput={(e: any) =>
                          handleChange('mikrotik_incident_sla_warn_minutes', e.target.value)}
                      />
                    </div>
                  </div>

                  <div class="setting-item">
                    <label for="mikrotik-sla-breach-minutes">
                      {$t('admin.settings.network.thresholds.incident_sla_breach_minutes') ||
                        'Incident SLA breach (minutes)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-sla-breach-minutes"
                        type="number"
                        min="1"
                        value={localSettings['mikrotik_incident_sla_breach_minutes']}
                        oninput={(e: any) =>
                          handleChange('mikrotik_incident_sla_breach_minutes', e.target.value)}
                      />
                    </div>
                  </div>

                  <div class="setting-item">
                    <label for="mikrotik-escalation-minutes">
                      {$t('admin.settings.network.thresholds.incident_escalation_minutes') ||
                        'Incident auto escalation (minutes)'}
                    </label>
                    <div class="setting-control">
                      <Input
                        id="mikrotik-escalation-minutes"
                        type="number"
                        min="5"
                        value={localSettings['mikrotik_incident_escalation_minutes']}
                        oninput={(e: any) =>
                          handleChange('mikrotik_incident_escalation_minutes', e.target.value)}
                      />
                    </div>
                  </div>
                </div>
              </div>

              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>
                    {$t('admin.settings.network.alerting.correlation_title') ||
                      'Incident correlation'}
                  </h3>
                  <p>
                    {$t('admin.settings.network.alerting.correlation_desc') ||
                      'Suppress CPU/latency incidents when offline incident is active as root cause.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['mikrotik_incident_correlation_enabled'] === 'true'}
                    onchange={(e) =>
                      handleChange(
                        'mikrotik_incident_correlation_enabled',
                        e.currentTarget.checked,
                      )}
                  />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>
                    {$t('admin.settings.network.alerting.auto_escalation_title') ||
                      'Auto escalation'}
                  </h3>
                  <p>
                    {$t('admin.settings.network.alerting.auto_escalation_desc') ||
                      'Escalate unacknowledged open incidents to critical after threshold minutes.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['mikrotik_incident_auto_escalation_enabled'] === 'true'}
                    onchange={(e) =>
                      handleChange(
                        'mikrotik_incident_auto_escalation_enabled',
                        e.currentTarget.checked,
                      )}
                  />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>{$t('admin.settings.network.alerting.email_title') || 'Email alerts'}</h3>
                  <p>
                    {$t('admin.settings.network.alerting.email_desc') ||
                      'Also send email to members who can access Network Routers.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['mikrotik_alert_email_enabled'] === 'true'}
                    onchange={(e) =>
                      handleChange('mikrotik_alert_email_enabled', e.currentTarget.checked)}
                  />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>
                    {$t('admin.settings.network.alerting.assignment_email_title') ||
                      'Incident assignment emails'}
                  </h3>
                  <p>
                    {$t('admin.settings.network.alerting.assignment_email_desc') ||
                      'Send email to the assigned member when incident assignee is changed.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['mikrotik_incident_assignment_email_enabled'] === 'true'}
                    onchange={(e) =>
                      handleChange(
                        'mikrotik_incident_assignment_email_enabled',
                        e.currentTarget.checked,
                      )}
                  />
                  <span class="slider"></span>
                </label>
              </div>

              <div class="setting-item setting-item-row mt-6">
                <div class="setting-info">
                  <h3>
                    {$t('admin.settings.network.alerting.pppoe_auto_apply_title') ||
                      'PPPoE auto-apply after save'}
                  </h3>
                  <p>
                    {$t('admin.settings.network.alerting.pppoe_auto_apply_desc') ||
                      'Automatically apply PPPoE create/update changes to MikroTik right after save.'}
                  </p>
                </div>
                <label class="toggle">
                  <input
                    type="checkbox"
                    checked={localSettings['pppoe_auto_apply_on_save_enabled'] === 'true'}
                    onchange={(e) =>
                      handleChange('pppoe_auto_apply_on_save_enabled', e.currentTarget.checked)}
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
              {#if SettingsEmailTabComponent}
                {@const EmailTab = SettingsEmailTabComponent}
                <EmailTab
                  {localSettings}
                  {emailProviderOptions}
                  {smtpEncryptionOptions}
                  bind:testEmailAddress
                  {sendingTestEmail}
                  {testingSmtp}
                  canReadEmailOutbox={$can('read', 'email_outbox')}
                  {handleChange}
                  onSendTestEmail={sendTestEmail}
                  onTestSmtpConnection={testSmtpConnection}
                  onViewOutbox={() => goto('../email-outbox')}
                />
              {:else}
                <div class="inline-tab-loading">
                  <div class="spinner"></div>
                </div>
              {/if}
            {:else if activeTab === 'payment'}
              {#if SettingsPaymentTabComponent}
                {@const PaymentTab = SettingsPaymentTabComponent}
                <PaymentTab
                  {localSettings}
                  {bankAccounts}
                  bind:newBank
                  bind:showAddBank
                  formattedLastRunAt={formatLastRunAt(localSettings['customer_invoice_last_run_at'])}
                  {handleChange}
                  {addBankAccount}
                  {removeBankAccount}
                />
              {:else}
                <div class="inline-tab-loading">
                  <div class="spinner"></div>
                </div>
              {/if}
            {:else if activeTab === 'whatsapp'}
              {#if SettingsWhatsAppTabComponent}
                {@const WhatsAppTab = SettingsWhatsAppTabComponent}
                <WhatsAppTab
                  {localSettings}
                  {handleChange}
                  eventScope="tenant"
                  title={$t('admin.settings.whatsapp.title') || 'WhatsApp Gateway'}
                  description={$t('admin.settings.whatsapp.description') ||
                    'Configure this tenant WhatsApp gateway.'}
                />
              {:else}
                <div class="inline-tab-loading" aria-busy={whatsappTabLoading}>
                  <div class="spinner"></div>
                </div>
              {/if}
            {:else if activeTab === 'event_notifications'}
              {#if SettingsNotificationEventsTabComponent}
                {@const NotificationEventsTab = SettingsNotificationEventsTabComponent}
                <NotificationEventsTab
                  {localSettings}
                  {handleChange}
                  eventSettingsKey="wa_events_tenant"
                  eventScope="tenant"
                  emailReady={emailVerificationReadiness.ready}
                  emailReadinessReason={emailVerificationReadiness.reason}
                  title={$t('admin.settings.event_notifications.title') || 'Event Notifications'}
                  description={$t('admin.settings.event_notifications.description') ||
                    'Choose notification channels for each tenant event.'}
                />
              {:else}
                <div class="inline-tab-loading" aria-busy={notificationEventsTabLoading}>
                  <div class="spinner"></div>
                </div>
              {/if}
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
      selectSettingsTab(e.detail);
      // Keep unsaved edits when switching tabs (avoid refetch/reset).
    }}
  />
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 1.5rem);
    max-width: 1400px;
    margin: 0 auto;
    --code-bg: rgba(255, 255, 255, 0.06);
  }

  :global([data-theme='light']) .page-container {
    --code-bg: rgba(0, 0, 0, 0.05);
  }
  .layout-grid {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 1.5rem;
    align-items: start;
  }

  .sidebar {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 0.75rem;
    position: sticky;
    top: 1.5rem;
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .sidebar {
    background: var(--bg-surface);
    box-shadow: var(--shadow-sm);
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
    background: rgba(99, 102, 241, 0.14);
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  :global([data-theme='light']) .nav-item.active {
    background: rgba(99, 102, 241, 0.1);
    border-color: rgba(99, 102, 241, 0.25);
  }

  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .card {
    background: var(--bg-surface);
    box-shadow: var(--shadow-sm);
  }
  .card-header {
    padding: 1.25rem 1.75rem;
    border-bottom: 1px solid var(--border-color);
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
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
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
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
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
  .warning-text {
    color: #f59e0b;
  }
  code {
    background: var(--code-bg);
    border: 1px solid var(--border-color);
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
    background: rgba(99, 102, 241, 0.08);
    border: 1px solid var(--color-primary-subtle);
    border-radius: var(--radius-lg);
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
    border-top: 1px solid var(--border-color);
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .logo-preview {
    width: 44px;
    height: 44px;
    object-fit: contain;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.02);
  }
  .loading-state {
    padding: 4rem;
    display: flex;
    justify-content: center;
  }
  .inline-tab-loading {
    min-height: 240px;
    display: flex;
    align-items: center;
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

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(255, 255, 255, 0.1);
    transition: 0.3s;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
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
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
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
    background: rgba(99, 102, 241, 0.12);
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .provider-card {
    background: rgba(255, 255, 255, 0.75);
  }

  :global([data-theme='light']) .provider-card:hover {
    background: rgba(99, 102, 241, 0.06);
  }

  :global([data-theme='light']) .provider-card.selected {
    background: rgba(99, 102, 241, 0.1);
    box-shadow: var(--shadow-sm);
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
    border: 1px solid var(--border-color);
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
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
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
  .mt-6 {
    margin-top: 1.5rem;
  }

  @media (max-width: 640px) {
    .config-grid {
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

</style>

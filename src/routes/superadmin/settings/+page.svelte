<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { Setting, BankAccount, EmailVerificationReadiness } from '$lib/api/client';
  import { isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import MobileFabMenu from '$lib/components/ui/MobileFabMenu.svelte';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import { superadminPlatformSettingsCache } from '$lib/stores/superadminPlatformSettings';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';

  // Modulariized Settings Components
  import SettingsGeneralTab from '$lib/components/superadmin/settings/SettingsGeneralTab.svelte';
  import SettingsAuthTab from '$lib/components/superadmin/settings/SettingsAuthTab.svelte';
  import SettingsPasswordTab from '$lib/components/superadmin/settings/SettingsPasswordTab.svelte';
  import SettingsSecurityTab from '$lib/components/superadmin/settings/SettingsSecurityTab.svelte';
  import SettingsStorageTab from '$lib/components/superadmin/settings/SettingsStorageTab.svelte';
  import SettingsPaymentTab from '$lib/components/superadmin/settings/SettingsPaymentTab.svelte';
  import SettingsAlertingTab from '$lib/components/superadmin/settings/SettingsAlertingTab.svelte';
  import SettingsBackupTab from '$lib/components/superadmin/settings/SettingsBackupTab.svelte';

  let loading = true;
  let saving = false;
  let activeTab = 'general';
  let isMobile = false;

  // Bank Account Data Models
  let bankAccounts: BankAccount[] = [];
  let newBankName = '';
  let newAccountNumber = '';
  let newAccountHolder = '';
  let addingBank = false;

  // Maintenance
  let maintenanceMode = false;
  let maintenanceMessage = '';

  // General
  let appPublicUrl = '';
  let appMainDomain = '';
  let currencyCode = 'IDR';
  const currencyCodeOptions = ['IDR', 'USD'];
  let appTimezone = 'UTC';

  // Authentication Settings
  let authAllowRegistration = false;
  let authRequireEmailVerification = false;
  let emailVerificationReadiness = { ready: true, reason: null } as EmailVerificationReadiness;
  let authJwtExpiryHours = 24;
  let authSessionTimeoutMinutes = 60;

  // Password Policy
  let authPasswordMinLength = 8;
  let authPasswordRequireUppercase = false;
  let authPasswordRequireNumber = false;
  let authPasswordRequireSpecial = false;
  let authLogoutAllOnPasswordChange = true;

  // Security & Rate Limiting
  let maxLoginAttempts = 5;
  let lockoutDurationMinutes = 15;
  let apiRateLimitPerMinute = 100;
  let enableIpBlocking = false;

  // 2FA Configuration
  let twoFAEnabled = true;
  let twoFAMethodTotp = true;
  let twoFAMethodEmail = false;
  let twoFAEmailOtpExpiryMinutes = 5;

  // Storage
  let storageMaxFileSizeMb = 500;
  let storageAllowedExtensions = '';

  // Storage Driver Config
  let storageDriver = 'local';
  let storageS3Bucket = '';
  let storageS3Region = 'us-east-1';
  let storageS3Endpoint = '';
  let storageS3AccessKey = '';
  let storageS3SecretKey = '';
  let storageS3PublicUrl = '';

  // Payment Settings
  let paymentMidtransEnabled = false;
  let paymentMidtransMerchantId = '';
  let paymentMidtransServerKey = '';
  let paymentMidtransClientKey = '';
  let paymentMidtransIsProduction = false;
  let paymentManualEnabled = true;
  let paymentManualInstructions = '';

  // Alerting Settings
  let alertingEnabled = false;
  let alertingEmail = '';
  let alertingErrorThreshold = 5.0;
  let alertingRateLimitThreshold = 50;
  let alertingResponseTimeThreshold = 3000;
  let alertingCooldownMinutes = 15;

  // Backup Settings
  let backupGlobalEnabled = false;
  type BackupMode = 'minute' | 'hour' | 'day' | 'week';
  type Weekday = 'mon' | 'tue' | 'wed' | 'thu' | 'fri' | 'sat' | 'sun';

  let backupGlobalMode: BackupMode = 'day';
  let backupGlobalEvery = 15;
  let backupGlobalAt = '02:00';
  let backupGlobalWeekday: Weekday = 'sun';
  let backupGlobalRetentionDays = 30;
  let backupTenantEnabled = false;
  let backupTenantMode: BackupMode = 'day';
  let backupTenantEvery = 60;
  let backupTenantAt = '02:30';
  let backupTenantWeekday: Weekday = 'sun';
  let backupTenantRetentionDays = 14;

  let hasChanges = false;

  // Sending test email state
  let testEmailAddress = '';
  let sendingTestEmail = false;

  const categories = {
    general: {
      labelKey: 'superadmin.settings.categories.general',
      labelFallback: 'General & Maintenance',
      icon: 'settings',
    },
    auth: {
      labelKey: 'superadmin.settings.categories.auth',
      labelFallback: 'Authentication',
      icon: 'lock',
    },
    password: {
      labelKey: 'superadmin.settings.categories.password',
      labelFallback: 'Password Policy',
      icon: 'key',
    },
    security: {
      labelKey: 'superadmin.settings.categories.security',
      labelFallback: 'Security & Rate Limiting',
      icon: 'shield',
    },
    storage: {
      labelKey: 'superadmin.settings.categories.storage',
      labelFallback: 'Storage Configuration',
      icon: 'hard-drive',
    },
    payment: {
      labelKey: 'superadmin.settings.categories.payment',
      labelFallback: 'Payment Gateway',
      icon: 'credit-card',
    },
    alerting: {
      labelKey: 'superadmin.settings.categories.alerting',
      labelFallback: 'Error Alerting',
      icon: 'bell',
    },
    backup: {
      labelKey: 'superadmin.settings.categories.backup',
      labelFallback: 'Backups',
      icon: 'archive',
    },
  };

  let pageTitle = 'Platform Settings';
  let pageSubtitle = 'Global Configuration';
  let categoryEntries: { id: string; icon: string; label: string }[] = [];

  $: pageTitle = $t('superadmin.settings.title') || 'Platform Settings';
  $: pageSubtitle = $t('superadmin.settings.subtitle') || 'Global Configuration';
  $: categoryEntries = Object.entries(categories).map(([id, cat]) => ({
    id,
    icon: cat.icon,
    label: $t(cat.labelKey) || cat.labelFallback,
  }));

  onMount(async () => {
    if (!$isSuperAdmin) {
      goto('/dashboard');
      return;
    }

    // Media query for responsive tweaks
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

    // Hydrate from cache to avoid spinner flash
    const cached = get(superadminPlatformSettingsCache);
    if (cached?.fetchedAt && Object.keys(cached.settingsMap || {}).length) {
      applySettingsMap(cached.settingsMap);
      bankAccounts = cached.bankAccounts || [];
      loading = false;
      // Background refresh (don’t block UI)
      void loadSettings({ silent: true });
      return;
    }

    await loadSettings();
  });

  function applySettingsMap(settingsMap: Record<string, string>) {
    // Maintenance
    maintenanceMode = settingsMap['maintenance_mode'] === 'true';
    maintenanceMessage =
      settingsMap['maintenance_message'] ||
      'The system is currently under maintenance. Please try again later.';

    // General
    appPublicUrl = settingsMap['app_public_url'] || 'http://localhost:3000';
    appMainDomain = settingsMap['app_main_domain'] || '';
    currencyCode = (settingsMap['currency_code'] || 'IDR').toUpperCase();
    if (currencyCode !== 'IDR' && currencyCode !== 'USD') {
      currencyCode = 'IDR';
    }
    appTimezone = settingsMap['app_timezone'] || 'UTC';

    // Authentication
    authAllowRegistration = settingsMap['auth_allow_registration'] === 'true';
    authRequireEmailVerification = settingsMap['auth_require_email_verification'] === 'true';
    authJwtExpiryHours = parseInt(settingsMap['auth_jwt_expiry_hours'] || '24');
    authSessionTimeoutMinutes = parseInt(settingsMap['auth_session_timeout_minutes'] || '60');

    // Password Policy
    authPasswordMinLength = parseInt(settingsMap['auth_password_min_length'] || '8');
    authPasswordRequireUppercase = settingsMap['auth_password_require_uppercase'] === 'true';
    authPasswordRequireNumber = settingsMap['auth_password_require_number'] === 'true';
    authPasswordRequireSpecial = settingsMap['auth_password_require_special'] === 'true';
    authLogoutAllOnPasswordChange = settingsMap['auth_logout_all_on_password_change'] !== 'false';

    // Security & Rate Limiting
    maxLoginAttempts = parseInt(
      settingsMap['auth_max_login_attempts'] || settingsMap['max_login_attempts'] || '5',
    );
    lockoutDurationMinutes = parseInt(
      settingsMap['auth_lockout_duration_minutes'] ||
        settingsMap['lockout_duration_minutes'] ||
        '15',
    );
    apiRateLimitPerMinute = parseInt(settingsMap['api_rate_limit_per_minute'] || '100');
    enableIpBlocking = settingsMap['enable_ip_blocking'] === 'true';

    // 2FA Configuration
    twoFAEnabled = settingsMap['2fa_enabled'] !== 'false'; // Default true
    const methods = settingsMap['2fa_methods'] || 'totp';
    twoFAMethodTotp = methods.includes('totp');
    twoFAMethodEmail = methods.includes('email');
    twoFAEmailOtpExpiryMinutes = parseInt(settingsMap['2fa_email_otp_expiry_minutes'] || '5');

    // Storage
    storageMaxFileSizeMb = parseInt(settingsMap['storage_max_file_size_mb'] || '500');
    storageAllowedExtensions =
      settingsMap['storage_allowed_extensions'] ||
      'jpg,jpeg,png,gif,pdf,doc,docx,xls,xlsx,zip,mp4,mov';

    storageDriver = settingsMap['storage_driver'] || 'local';
    storageS3Bucket = settingsMap['storage_s3_bucket'] || '';
    storageS3Region = settingsMap['storage_s3_region'] || 'auto';
    storageS3Endpoint = settingsMap['storage_s3_endpoint'] || '';
    storageS3AccessKey = settingsMap['storage_s3_access_key'] || '';
    storageS3SecretKey = settingsMap['storage_s3_secret_key'] || '';
    storageS3PublicUrl = settingsMap['storage_s3_public_url'] || '';

    // Payment
    paymentMidtransEnabled = settingsMap['payment_midtrans_enabled'] === 'true';
    paymentMidtransMerchantId = settingsMap['payment_midtrans_merchant_id'] || '';
    paymentMidtransServerKey = settingsMap['payment_midtrans_server_key'] || '';
    paymentMidtransClientKey = settingsMap['payment_midtrans_client_key'] || '';
    paymentMidtransIsProduction = settingsMap['payment_midtrans_is_production'] === 'true';
    paymentManualEnabled = settingsMap['payment_manual_enabled'] !== 'false'; // Default true
    paymentManualInstructions =
      settingsMap['payment_manual_instructions'] || 'Please transfer to our bank account.';

    // Alerting
    alertingEnabled = settingsMap['alerting_enabled'] === 'true';
    alertingEmail = settingsMap['alerting_email'] || '';
    alertingErrorThreshold = parseFloat(settingsMap['alerting_error_threshold'] || '5.0');
    alertingRateLimitThreshold = parseInt(settingsMap['alerting_rate_limit_threshold'] || '50');
    alertingResponseTimeThreshold = parseInt(
      settingsMap['alerting_response_time_threshold'] || '3000',
    );
    alertingCooldownMinutes = parseInt(settingsMap['alerting_cooldown_minutes'] || '15');

    // Backup
    backupGlobalEnabled = settingsMap['backup_global_enabled'] === 'true';
    backupTenantEnabled = settingsMap['backup_tenant_enabled'] === 'true';
    backupGlobalMode = (settingsMap['backup_global_mode'] as BackupMode) || 'day';
    backupGlobalEvery = parseInt(settingsMap['backup_global_every'] || '15');
    backupGlobalAt = settingsMap['backup_global_at'] || '02:00';
    backupGlobalWeekday = (settingsMap['backup_global_weekday'] as Weekday) || 'sun';
    backupGlobalRetentionDays = parseInt(settingsMap['backup_global_retention_days'] || '30');

    backupTenantMode = (settingsMap['backup_tenant_mode'] as BackupMode) || 'day';
    backupTenantEvery = parseInt(settingsMap['backup_tenant_every'] || '60');
    backupTenantAt = settingsMap['backup_tenant_at'] || '02:30';
    backupTenantWeekday = (settingsMap['backup_tenant_weekday'] as Weekday) || 'sun';
    backupTenantRetentionDays = parseInt(settingsMap['backup_tenant_retention_days'] || '14');
  }

  async function loadSettings(opts: { silent?: boolean } = {}) {
    if (!opts.silent) loading = true;
    try {
      const [data, readiness] = await Promise.all([
        api.settings.getAll(),
        api.settings
          .getEmailVerificationReadiness()
          .catch(() => ({ ready: true, reason: null }) as EmailVerificationReadiness),
      ]);
      const settingsMap: Record<string, string> = {};
      data.forEach((s) => {
        settingsMap[s.key] = s.value;
      });
      emailVerificationReadiness = readiness;

      applySettingsMap(settingsMap);

      // Load banks (always load to ensure availability)
      try {
        bankAccounts = await api.payment.listBanks();
      } catch (e) {
        console.error('Failed to load banks:', e);
      }

      superadminPlatformSettingsCache.set({
        settingsMap,
        bankAccounts,
        fetchedAt: Date.now(),
      });
    } catch (err) {
      console.error('Failed to load settings:', err);
      toast.error(get(t)('superadmin.settings.errors.load_failed') || 'Failed to load settings');
    } finally {
      if (!opts.silent) loading = false;
    }
  }

  function handleChange() {
    hasChanges = true;
  }

  async function saveSettings() {
    if (authRequireEmailVerification && !emailVerificationReadiness.ready) {
      toast.error(
        emailVerificationReadiness.reason ||
          get(t)('superadmin.settings.auth.require_email_verification.not_ready') ||
          'Email configuration is not ready. Configure email provider first.',
      );
      return;
    }

    saving = true;
    try {
      const updates = [
        // Maintenance
        api.settings.upsert('app_public_url', appPublicUrl, 'Public Application URL'),
        api.settings.upsert('app_main_domain', appMainDomain, 'Main Application Domain'),
        api.settings.upsert(
          'currency_code',
          currencyCode.toUpperCase(),
          'Default currency code (ISO 4217)',
        ),
        api.settings.upsert(
          'app_timezone',
          appTimezone,
          'Application timezone for schedules (IANA, e.g. Asia/Jakarta)',
        ),
        api.settings.upsert(
          'maintenance_mode',
          maintenanceMode ? 'true' : 'false',
          'Global maintenance mode',
        ),
        api.settings.upsert(
          'maintenance_message',
          maintenanceMessage,
          'Message shown during maintenance',
        ),
        // Authentication
        api.settings.upsert(
          'auth_allow_registration',
          authAllowRegistration ? 'true' : 'false',
          'Allow public registration',
        ),
        api.settings.upsert(
          'auth_require_email_verification',
          authRequireEmailVerification ? 'true' : 'false',
          'Require email verification before login',
        ),
        api.settings.upsert(
          'auth_jwt_expiry_hours',
          authJwtExpiryHours.toString(),
          'JWT token expiry in hours',
        ),
        api.settings.upsert(
          'auth_session_timeout_minutes',
          authSessionTimeoutMinutes.toString(),
          'Inactivity session timeout in minutes',
        ),
        // Password Policy
        api.settings.upsert(
          'auth_password_min_length',
          authPasswordMinLength.toString(),
          'Minimum password length',
        ),
        api.settings.upsert(
          'auth_password_require_uppercase',
          authPasswordRequireUppercase ? 'true' : 'false',
          'Require uppercase letter in password',
        ),
        api.settings.upsert(
          'auth_password_require_number',
          authPasswordRequireNumber ? 'true' : 'false',
          'Require number in password',
        ),
        api.settings.upsert(
          'auth_password_require_special',
          authPasswordRequireSpecial ? 'true' : 'false',
          'Require special character in password',
        ),
        api.settings.upsert(
          'auth_logout_all_on_password_change',
          authLogoutAllOnPasswordChange ? 'true' : 'false',
          'Logout all sessions on password change',
        ),
        // Security
        api.settings.upsert(
          'auth_max_login_attempts',
          maxLoginAttempts.toString(),
          'Maximum failed login attempts',
        ),
        api.settings.upsert(
          'auth_lockout_duration_minutes',
          lockoutDurationMinutes.toString(),
          'Lockout duration in minutes',
        ),
        api.settings.upsert(
          'api_rate_limit_per_minute',
          apiRateLimitPerMinute.toString(),
          'API rate limit per minute',
        ),
        api.settings.upsert(
          'enable_ip_blocking',
          enableIpBlocking ? 'true' : 'false',
          'Enable IP blocking',
        ),
        // 2FA Configuration
        api.settings.upsert(
          '2fa_enabled',
          twoFAEnabled ? 'true' : 'false',
          'Enable Two-Factor Authentication',
        ),
        api.settings.upsert(
          '2fa_methods',
          [twoFAMethodTotp ? 'totp' : '', twoFAMethodEmail ? 'email' : '']
            .filter(Boolean)
            .join(',') || 'totp',
          'Available 2FA methods',
        ),
        api.settings.upsert(
          '2fa_email_otp_expiry_minutes',
          twoFAEmailOtpExpiryMinutes.toString(),
          'Email OTP expiry in minutes',
        ),
        // Storage
        api.settings.upsert(
          'storage_max_file_size_mb',
          storageMaxFileSizeMb.toString(),
          'Maximum file upload size in MB',
        ),
        api.settings.upsert(
          'storage_allowed_extensions',
          storageAllowedExtensions,
          'Allowed file extensions',
        ),
        // Storage Driver
        api.settings.upsert('storage_driver', storageDriver, 'Storage Driver'),
        api.settings.upsert('storage_s3_bucket', storageS3Bucket, 'S3 Bucket'),
        api.settings.upsert('storage_s3_region', storageS3Region, 'S3 Region'),
        api.settings.upsert('storage_s3_endpoint', storageS3Endpoint, 'S3 Endpoint'),
        api.settings.upsert('storage_s3_access_key', storageS3AccessKey, 'S3 Access Key'),
        api.settings.upsert('storage_s3_secret_key', storageS3SecretKey, 'S3 Secret Key'),
        api.settings.upsert('storage_s3_public_url', storageS3PublicUrl, 'S3 Public URL'),
        // Payment
        api.settings.upsert(
          'payment_midtrans_enabled',
          paymentMidtransEnabled ? 'true' : 'false',
          'Enable Midtrans',
        ),
        api.settings.upsert(
          'payment_midtrans_merchant_id',
          paymentMidtransMerchantId,
          'Midtrans Merchant ID',
        ),
        api.settings.upsert(
          'payment_midtrans_server_key',
          paymentMidtransServerKey,
          'Midtrans Server Key',
        ),
        api.settings.upsert(
          'payment_midtrans_client_key',
          paymentMidtransClientKey,
          'Midtrans Client Key',
        ),
        api.settings.upsert(
          'payment_midtrans_is_production',
          paymentMidtransIsProduction ? 'true' : 'false',
          'Midtrans Production Mode',
        ),
        api.settings.upsert(
          'payment_manual_enabled',
          paymentManualEnabled ? 'true' : 'false',
          'Enable Manual Payment',
        ),
        api.settings.upsert(
          'payment_manual_instructions',
          paymentManualInstructions,
          'Manual Payment Instructions',
        ),
        // Alerting
        api.settings.upsert(
          'alerting_enabled',
          alertingEnabled ? 'true' : 'false',
          'Enable error alerting via email',
        ),
        api.settings.upsert('alerting_email', alertingEmail, 'Email address to receive alerts'),
        api.settings.upsert(
          'alerting_error_threshold',
          alertingErrorThreshold.toString(),
          'Error rate threshold percentage',
        ),
        api.settings.upsert(
          'alerting_rate_limit_threshold',
          alertingRateLimitThreshold.toString(),
          'Rate limit count threshold',
        ),
        api.settings.upsert(
          'alerting_response_time_threshold',
          alertingResponseTimeThreshold.toString(),
          'P95 response time threshold in ms',
        ),
        api.settings.upsert(
          'alerting_cooldown_minutes',
          alertingCooldownMinutes.toString(),
          'Minutes between same alert type',
        ),
        // Backups
        api.settings.upsert(
          'backup_global_enabled',
          backupGlobalEnabled ? 'true' : 'false',
          'Enable automatic global backups',
        ),
        api.settings.upsert(
          'backup_global_mode',
          backupGlobalMode,
          'Global backup schedule mode: minute, hour, day, week',
        ),
        api.settings.upsert(
          'backup_global_every',
          backupGlobalEvery.toString(),
          'Global backup interval value for minute/hour modes',
        ),
        api.settings.upsert(
          'backup_global_at',
          backupGlobalAt,
          'Global backup time (HH:MM) for day/week modes (app_timezone)',
        ),
        api.settings.upsert(
          'backup_global_weekday',
          backupGlobalWeekday,
          'Global backup weekday for weekly mode (mon..sun)',
        ),
        api.settings.upsert(
          'backup_global_retention_days',
          backupGlobalRetentionDays.toString(),
          'Retention days for global backups',
        ),
        api.settings.upsert(
          'backup_tenant_enabled',
          backupTenantEnabled ? 'true' : 'false',
          'Enable automatic tenant backups',
        ),
        api.settings.upsert(
          'backup_tenant_mode',
          backupTenantMode,
          'Tenant backup schedule mode: minute, hour, day, week',
        ),
        api.settings.upsert(
          'backup_tenant_every',
          backupTenantEvery.toString(),
          'Tenant backup interval value for minute/hour modes',
        ),
        api.settings.upsert(
          'backup_tenant_at',
          backupTenantAt,
          'Tenant backup time (HH:MM) for day/week modes (app_timezone)',
        ),
        api.settings.upsert(
          'backup_tenant_weekday',
          backupTenantWeekday,
          'Tenant backup weekday for weekly mode (mon..sun)',
        ),
        api.settings.upsert(
          'backup_tenant_retention_days',
          backupTenantRetentionDays.toString(),
          'Retention days for tenant backups',
        ),
      ];

      await Promise.all(updates);

      toast.success(get(t)('superadmin.settings.toasts.saved') || 'Settings saved successfully');
      hasChanges = false;

      // Refresh global settings store so currency/locale update immediately
      await appSettings.refresh();

      // Refresh cache with latest values
      const settingsMap: Record<string, string> = {
        app_public_url: appPublicUrl,
        app_main_domain: appMainDomain,
        currency_code: currencyCode.toUpperCase(),
        app_timezone: appTimezone,
        maintenance_mode: maintenanceMode ? 'true' : 'false',
        maintenance_message: maintenanceMessage,
        auth_allow_registration: authAllowRegistration ? 'true' : 'false',
        auth_require_email_verification: authRequireEmailVerification ? 'true' : 'false',
        auth_jwt_expiry_hours: authJwtExpiryHours.toString(),
        auth_session_timeout_minutes: authSessionTimeoutMinutes.toString(),
        auth_password_min_length: authPasswordMinLength.toString(),
        auth_password_require_uppercase: authPasswordRequireUppercase ? 'true' : 'false',
        auth_password_require_number: authPasswordRequireNumber ? 'true' : 'false',
        auth_password_require_special: authPasswordRequireSpecial ? 'true' : 'false',
        auth_logout_all_on_password_change: authLogoutAllOnPasswordChange ? 'true' : 'false',
        auth_max_login_attempts: maxLoginAttempts.toString(),
        auth_lockout_duration_minutes: lockoutDurationMinutes.toString(),
        api_rate_limit_per_minute: apiRateLimitPerMinute.toString(),
        enable_ip_blocking: enableIpBlocking ? 'true' : 'false',
        '2fa_enabled': twoFAEnabled ? 'true' : 'false',
        '2fa_methods': [twoFAMethodTotp ? 'totp' : null, twoFAMethodEmail ? 'email' : null]
          .filter(Boolean)
          .join(','),
        '2fa_email_otp_expiry_minutes': twoFAEmailOtpExpiryMinutes.toString(),
        storage_max_file_size_mb: storageMaxFileSizeMb.toString(),
        storage_allowed_extensions: storageAllowedExtensions,
        storage_driver: storageDriver,
        storage_s3_bucket: storageS3Bucket,
        storage_s3_region: storageS3Region,
        storage_s3_endpoint: storageS3Endpoint,
        storage_s3_access_key: storageS3AccessKey,
        storage_s3_secret_key: storageS3SecretKey,
        storage_s3_public_url: storageS3PublicUrl,
        payment_midtrans_enabled: paymentMidtransEnabled ? 'true' : 'false',
        payment_midtrans_merchant_id: paymentMidtransMerchantId,
        payment_midtrans_server_key: paymentMidtransServerKey,
        payment_midtrans_client_key: paymentMidtransClientKey,
        payment_midtrans_is_production: paymentMidtransIsProduction ? 'true' : 'false',
        payment_manual_enabled: paymentManualEnabled ? 'true' : 'false',
        payment_manual_instructions: paymentManualInstructions,
        alerting_enabled: alertingEnabled ? 'true' : 'false',
        alerting_email: alertingEmail,
        alerting_error_threshold: alertingErrorThreshold.toString(),
        alerting_rate_limit_threshold: alertingRateLimitThreshold.toString(),
        alerting_response_time_threshold: alertingResponseTimeThreshold.toString(),
        alerting_cooldown_minutes: alertingCooldownMinutes.toString(),
        backup_global_enabled: backupGlobalEnabled ? 'true' : 'false',
        backup_global_mode: backupGlobalMode,
        backup_global_every: backupGlobalEvery.toString(),
        backup_global_at: backupGlobalAt,
        backup_global_weekday: backupGlobalWeekday,
        backup_global_retention_days: backupGlobalRetentionDays.toString(),
        backup_tenant_enabled: backupTenantEnabled ? 'true' : 'false',
        backup_tenant_mode: backupTenantMode,
        backup_tenant_every: backupTenantEvery.toString(),
        backup_tenant_at: backupTenantAt,
        backup_tenant_weekday: backupTenantWeekday,
        backup_tenant_retention_days: backupTenantRetentionDays.toString(),
      };

      superadminPlatformSettingsCache.set({
        settingsMap,
        bankAccounts,
        fetchedAt: Date.now(),
      });
    } catch (err) {
      console.error('Failed to save settings:', err);
      toast.error(get(t)('superadmin.settings.errors.save_failed') || 'Failed to save settings');
    } finally {
      saving = false;
    }
  }

  function discardChanges() {
    void loadSettings();
    hasChanges = false;
  }

  async function sendTestEmail() {
    if (!testEmailAddress) {
      toast.error(
        get(t)('superadmin.settings.errors.missing_test_email') || 'Please enter an email address',
      );
      return;
    }
    sendingTestEmail = true;
    try {
      const result = await api.settings.sendTestEmail(testEmailAddress);
      toast.success(String(result));
    } catch (error) {
      console.error(error);
      toast.error(
        (get(t)('superadmin.settings.errors.test_email_failed') || 'Failed to send test email: ') +
          String(error),
      );
    } finally {
      sendingTestEmail = false;
    }
  }

  async function addBank() {
    if (!newBankName || !newAccountNumber || !newAccountHolder) return;
    addingBank = true;
    try {
      await api.payment.createBank(newBankName, newAccountNumber, newAccountHolder);
      bankAccounts = await api.payment.listBanks();
      newBankName = '';
      newAccountNumber = '';
      newAccountHolder = '';
      toast.success(get(t)('superadmin.settings.toasts.bank_added') || 'Bank account added');
    } catch (e: any) {
      toast.error(
        e.message || get(t)('superadmin.settings.errors.bank_add_failed') || 'Failed to add bank',
      );
    } finally {
      addingBank = false;
    }
  }

  async function deleteBank(id: string) {
    if (!confirm(get(t)('superadmin.settings.confirm.are_you_sure') || 'Are you sure?')) return;
    try {
      await api.payment.deleteBank(id);
      bankAccounts = bankAccounts.filter((b) => b.id !== id);
      toast.success(get(t)('superadmin.settings.toasts.bank_removed') || 'Bank account removed');
    } catch (e: any) {
      toast.error(
        e.message ||
          get(t)('superadmin.settings.errors.bank_remove_failed') ||
          'Failed to delete bank',
      );
    }
  }

  function showMessage(type: 'success' | 'error', msg: string) {
    if (type === 'success') toast.success(msg);
    else toast.error(msg);
  }

  async function triggerGlobalBackup() {
    try {
      await api.settings.upsert(
        'backup_global_trigger',
        'true',
        'Manual trigger for global backup',
      );
      toast.success('Global backup queued');
    } catch (err) {
      console.error('Failed to trigger global backup:', err);
      toast.error('Failed to queue global backup');
    }
  }

  async function triggerTenantBackups() {
    try {
      await api.settings.upsert(
        'backup_tenant_trigger',
        'true',
        'Manual trigger for tenant backups',
      );
      toast.success('Tenant backups queued');
    } catch (err) {
      console.error('Failed to trigger tenant backups:', err);
      toast.error('Failed to queue tenant backups');
    }
  }
</script>

<div class="page-container fade-in">
  <div class="layout-grid">
    <!-- Desktop Sidebar -->
    <aside class="sidebar card desktop-sidebar">
      <nav>
        {#each categoryEntries as cat}
          <button
            class="nav-item {activeTab === cat.id ? 'active' : ''}"
            on:click={() => {
              activeTab = cat.id;
            }}
          >
            <span class="icon">
              <Icon name={cat.icon} size={18} />
            </span>
            {cat.label}
          </button>
        {/each}
      </nav>
    </aside>

    <!-- Mobile FAB & Menu -->
    <MobileFabMenu
      items={categoryEntries.map((cat) => ({
        id: cat.id,
        label: cat.label,
        icon: cat.icon,
      }))}
      bind:activeTab
      title={pageTitle}
    />

    <main class="content">
      <div class="header-mobile">
        <h1>{pageTitle}</h1>
        <p class="subtitle">{pageSubtitle}</p>
      </div>

      {#if loading}
        <div class="loading-state">
          <div class="spinner"></div>
          <p>
            {$t('superadmin.settings.loading') || 'Loading settings...'}
          </p>
        </div>
      {:else}
        <!-- General & Maintenance Tab -->
        {#if activeTab === 'general'}
          <SettingsGeneralTab
            bind:appPublicUrl
            bind:appMainDomain
            bind:currencyCode
            bind:appTimezone
            {currencyCodeOptions}
            bind:maintenanceMode
            bind:maintenanceMessage
            on:change={handleChange}
          />
        {/if}

        <!-- Authentication Tab -->
        {#if activeTab === 'auth'}
          <SettingsAuthTab
            bind:authAllowRegistration
            bind:authRequireEmailVerification
            emailVerificationReady={emailVerificationReadiness.ready}
            emailVerificationReason={emailVerificationReadiness.reason || ''}
            bind:authJwtExpiryHours
            bind:authSessionTimeoutMinutes
            on:change={handleChange}
          />
        {/if}

        <!-- Password Policy Tab -->
        {#if activeTab === 'password'}
          <SettingsPasswordTab
            bind:authPasswordMinLength
            bind:authPasswordRequireUppercase
            bind:authPasswordRequireNumber
            bind:authPasswordRequireSpecial
            on:change={handleChange}
          />
        {/if}

        <!-- Security Tab -->
        {#if activeTab === 'security'}
          <SettingsSecurityTab
            bind:maxLoginAttempts
            bind:lockoutDurationMinutes
            bind:apiRateLimitPerMinute
            bind:enableIpBlocking
            bind:twoFAEnabled
            bind:twoFAMethodTotp
            bind:twoFAMethodEmail
            bind:twoFAEmailOtpExpiryMinutes
            on:change={handleChange}
          />
        {/if}

        <!-- Storage Tab -->
        {#if activeTab === 'storage'}
          <SettingsStorageTab
            bind:storageDriver
            bind:storageS3Bucket
            bind:storageS3Region
            bind:storageS3Endpoint
            bind:storageS3AccessKey
            bind:storageS3SecretKey
            bind:storageS3PublicUrl
            bind:storageMaxFileSizeMb
            bind:storageAllowedExtensions
            on:change={handleChange}
          />
        {/if}

        <!-- Payment Tab -->
        {#if activeTab === 'payment'}
          <SettingsPaymentTab
            bind:paymentMidtransEnabled
            bind:paymentMidtransMerchantId
            bind:paymentMidtransServerKey
            bind:paymentMidtransClientKey
            bind:paymentMidtransIsProduction
            bind:paymentManualEnabled
            bind:paymentManualInstructions
            {bankAccounts}
            bind:newBankName
            bind:newAccountNumber
            bind:newAccountHolder
            {addingBank}
            {isMobile}
            on:change={handleChange}
            on:addBank={addBank}
            on:deleteBank={(e) => deleteBank(e.detail)}
          />
        {/if}

        <!-- Alerting Tab -->
        {#if activeTab === 'alerting'}
          <SettingsAlertingTab
            bind:alertingEnabled
            bind:alertingEmail
            bind:alertingErrorThreshold
            bind:alertingRateLimitThreshold
            bind:alertingResponseTimeThreshold
            bind:alertingCooldownMinutes
            on:change={handleChange}
          />
        {/if}

        <!-- Backup Tab -->
        {#if activeTab === 'backup'}
          <SettingsBackupTab
            {appTimezone}
            bind:backupGlobalEnabled
            bind:backupGlobalMode
            bind:backupGlobalEvery
            bind:backupGlobalAt
            bind:backupGlobalWeekday
            bind:backupGlobalRetentionDays
            bind:backupTenantEnabled
            bind:backupTenantMode
            bind:backupTenantEvery
            bind:backupTenantAt
            bind:backupTenantWeekday
            bind:backupTenantRetentionDays
            on:change={handleChange}
            on:triggerGlobal={triggerGlobalBackup}
            on:triggerTenants={triggerTenantBackups}
          />
        {/if}

        <!-- Actions Footer -->
        <div class="actions-footer">
          <button
            class="btn btn-secondary"
            disabled={!hasChanges || saving}
            on:click={discardChanges}
          >
            {$t('superadmin.settings.actions.reset') || 'Reset'}
          </button>
          <button class="btn btn-primary" on:click={saveSettings} disabled={!hasChanges || saving}>
            {#if saving}
              <div class="spinner-sm"></div>
              {$t('superadmin.settings.actions.saving') || 'Saving...'}
            {:else}
              <Icon name="save" size={18} />
              {$t('superadmin.settings.actions.save') || 'Save Changes'}
            {/if}
          </button>
        </div>
      {/if}
    </main>
  </div>
</div>

<style>
  .page-container {
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
  }

  .layout-grid {
    display: grid;
    grid-template-columns: 260px 1fr;
    gap: 2rem;
    align-items: start;
  }

  .sidebar {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1rem;
    position: sticky;
    top: 2rem;
    z-index: 10;
    height: fit-content;
  }

  .sidebar nav {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.95rem;
    font-weight: 500;
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all 0.2s;
    text-align: left;
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  .header-mobile {
    margin-bottom: 1.5rem;
  }

  .header-mobile h1 {
    font-size: 1.8rem;
    font-weight: 700;
    margin: 0 0 0.5rem 0;
    color: var(--text-primary);
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin: 0;
  }

  .card {
    background: var(--bg-surface);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
    margin-bottom: 1.5rem;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    color: var(--text-secondary);
    gap: 1rem;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .spinner-sm {
    width: 18px;
    height: 18px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Actions Footer */
  .actions-footer {
    margin-top: 2rem;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 1rem;
    position: sticky;
    bottom: 0px;
    padding: 1.5rem 2rem;
    background: var(--bg-surface);
    border-top: 1px solid var(--border-color);
    z-index: 100;
    /* Negative margins to span full width of container padding */
    margin-left: -2rem;
    margin-right: -2rem;
    margin-bottom: -2rem;
    width: calc(100% + 4rem);
    box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.2);
  }

  /* Adjust for mobile */
  @media (max-width: 600px) {
    .actions-footer {
      padding: 1rem;
      margin-left: -1rem;
      margin-right: -1rem;
      margin-bottom: -1rem;
      width: calc(100% + 2rem);
    }
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
    transition: all 0.2s ease;
    border: 1px solid transparent;
    min-width: 100px;
  }

  .btn:active {
    transform: scale(0.98);
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
    box-shadow: 0 2px 4px rgba(var(--color-primary-rgb), 0.3);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
    box-shadow: 0 4px 8px rgba(var(--color-primary-rgb), 0.4);
    transform: translateY(-1px);
  }

  .btn-secondary {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-color: var(--text-secondary);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none !important;
    box-shadow: none !important;
  }

  /* Mobile Responsive */
  @media (max-width: 900px) {
    .layout-grid {
      grid-template-columns: 1fr;
      gap: 0;
    }

    .desktop-sidebar {
      display: none;
    }

    .page-container {
      padding: 1rem;
    }

    .header-mobile h1 {
      font-size: 1.5rem;
    }
  }
</style>

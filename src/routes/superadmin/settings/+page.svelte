<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import type { Setting, BankAccount } from "$lib/api/client";
    import { isSuperAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import Icon from "$lib/components/Icon.svelte";
    import MobileFabMenu from "$lib/components/MobileFabMenu.svelte";
    import { toast } from "$lib/stores/toast";
    import { appSettings } from "$lib/stores/settings";
    import { superadminPlatformSettingsCache } from "$lib/stores/superadminPlatformSettings";
    import { get } from "svelte/store";
    import { t } from "svelte-i18n";

    let loading = true;
    let saving = false;
    let activeTab = "general";
    let isMobile = false;

    // Bank Account Data Models
    let bankAccounts: BankAccount[] = [];
    let newBankName = "";
    let newAccountNumber = "";
    let newAccountHolder = "";
    let addingBank = false;

    // Maintenance
    let maintenanceMode = false;
    let maintenanceMessage = "";

    // General
    let appPublicUrl = "";
    let currencyCode = "IDR";
    const currencyCodeOptions = [
        "IDR",
        "USD",
        "EUR",
        "SGD",
        "MYR",
        "GBP",
        "JPY",
        "AUD",
    ];

    // Authentication Settings
    let authAllowRegistration = false;
    let authRequireEmailVerification = false;
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
    let storageAllowedExtensions = "";

    // Storage Driver Config
    let storageDriver = "local";
    let storageS3Bucket = "";
    let storageS3Region = "us-east-1";
    let storageS3Endpoint = "";
    let storageS3AccessKey = "";
    let storageS3SecretKey = "";
    let storageS3PublicUrl = "";

    // Payment Settings
    let paymentMidtransEnabled = false;
    let paymentMidtransMerchantId = "";
    let paymentMidtransServerKey = "";
    let paymentMidtransClientKey = "";
    let paymentMidtransIsProduction = false;
    let paymentManualEnabled = true;
    let paymentManualInstructions = "";

    let hasChanges = false;

    // Sending test email state
    let testEmailAddress = "";
    let sendingTestEmail = false;

    const categories = {
        general: {
            labelKey: "superadmin.settings.categories.general",
            labelFallback: "General & Maintenance",
            icon: "settings",
        },
        auth: {
            labelKey: "superadmin.settings.categories.auth",
            labelFallback: "Authentication",
            icon: "lock",
        },
        password: {
            labelKey: "superadmin.settings.categories.password",
            labelFallback: "Password Policy",
            icon: "key",
        },
        security: {
            labelKey: "superadmin.settings.categories.security",
            labelFallback: "Security & Rate Limiting",
            icon: "shield",
        },
        storage: {
            labelKey: "superadmin.settings.categories.storage",
            labelFallback: "Storage Configuration",
            icon: "hard-drive",
        },
        payment: {
            labelKey: "superadmin.settings.categories.payment",
            labelFallback: "Payment Gateway",
            icon: "credit-card",
        },
    };

    let pageTitle = "Platform Settings";
    let pageSubtitle = "Global Configuration";
    let categoryEntries: { id: string; icon: string; label: string }[] = [];

    $: pageTitle = $t("superadmin.settings.title") || "Platform Settings";
    $: pageSubtitle =
        $t("superadmin.settings.subtitle") || "Global Configuration";
    $: categoryEntries = Object.entries(categories).map(([id, cat]) => ({
        id,
        icon: cat.icon,
        label: $t(cat.labelKey) || cat.labelFallback,
    }));

    onMount(async () => {
        if (!$isSuperAdmin) {
            goto("/dashboard");
            return;
        }

        // Media query for responsive tweaks
        if (typeof window !== "undefined") {
            const mq = window.matchMedia("(max-width: 900px)");
            const sync = () => {
                isMobile = mq.matches;
            };
            sync();
            try {
                mq.addEventListener("change", sync);
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
        maintenanceMode = settingsMap["maintenance_mode"] === "true";
        maintenanceMessage =
            settingsMap["maintenance_message"] ||
            "The system is currently under maintenance. Please try again later.";

        // General
        appPublicUrl = settingsMap["app_public_url"] || "http://localhost:3000";
        currencyCode = (settingsMap["currency_code"] || "IDR").toUpperCase();

        // Authentication
        authAllowRegistration = settingsMap["auth_allow_registration"] === "true";
        authRequireEmailVerification =
            settingsMap["auth_require_email_verification"] === "true";
        authJwtExpiryHours = parseInt(settingsMap["auth_jwt_expiry_hours"] || "24");
        authSessionTimeoutMinutes = parseInt(
            settingsMap["auth_session_timeout_minutes"] || "60",
        );

        // Password Policy
        authPasswordMinLength = parseInt(
            settingsMap["auth_password_min_length"] || "8",
        );
        authPasswordRequireUppercase =
            settingsMap["auth_password_require_uppercase"] === "true";
        authPasswordRequireNumber =
            settingsMap["auth_password_require_number"] === "true";
        authPasswordRequireSpecial =
            settingsMap["auth_password_require_special"] === "true";
        authLogoutAllOnPasswordChange =
            settingsMap["auth_logout_all_on_password_change"] !== "false";

        // Security & Rate Limiting
        maxLoginAttempts = parseInt(
            settingsMap["auth_max_login_attempts"] ||
                settingsMap["max_login_attempts"] ||
                "5",
        );
        lockoutDurationMinutes = parseInt(
            settingsMap["auth_lockout_duration_minutes"] ||
                settingsMap["lockout_duration_minutes"] ||
                "15",
        );
        apiRateLimitPerMinute = parseInt(
            settingsMap["api_rate_limit_per_minute"] || "100",
        );
        enableIpBlocking = settingsMap["enable_ip_blocking"] === "true";

        // 2FA Configuration
        twoFAEnabled = settingsMap["2fa_enabled"] !== "false"; // Default true
        const methods = settingsMap["2fa_methods"] || "totp";
        twoFAMethodTotp = methods.includes("totp");
        twoFAMethodEmail = methods.includes("email");
        twoFAEmailOtpExpiryMinutes = parseInt(
            settingsMap["2fa_email_otp_expiry_minutes"] || "5",
        );

        // Storage
        storageMaxFileSizeMb = parseInt(
            settingsMap["storage_max_file_size_mb"] || "500",
        );
        storageAllowedExtensions =
            settingsMap["storage_allowed_extensions"] ||
            "jpg,jpeg,png,gif,pdf,doc,docx,xls,xlsx,zip,mp4,mov";

        storageDriver = settingsMap["storage_driver"] || "local";
        storageS3Bucket = settingsMap["storage_s3_bucket"] || "";
        storageS3Region = settingsMap["storage_s3_region"] || "auto";
        storageS3Endpoint = settingsMap["storage_s3_endpoint"] || "";
        storageS3AccessKey = settingsMap["storage_s3_access_key"] || "";
        storageS3SecretKey = settingsMap["storage_s3_secret_key"] || "";
        storageS3PublicUrl = settingsMap["storage_s3_public_url"] || "";

        // Payment
        paymentMidtransEnabled = settingsMap["payment_midtrans_enabled"] === "true";
        paymentMidtransMerchantId = settingsMap["payment_midtrans_merchant_id"] || "";
        paymentMidtransServerKey = settingsMap["payment_midtrans_server_key"] || "";
        paymentMidtransClientKey = settingsMap["payment_midtrans_client_key"] || "";
        paymentMidtransIsProduction =
            settingsMap["payment_midtrans_is_production"] === "true";
        paymentManualEnabled = settingsMap["payment_manual_enabled"] !== "false"; // Default true
        paymentManualInstructions =
            settingsMap["payment_manual_instructions"] ||
            "Please transfer to our bank account.";
    }

    async function loadSettings(opts: { silent?: boolean } = {}) {
        if (!opts.silent) loading = true;
        try {
            const data = await api.settings.getAll();
            const settingsMap: Record<string, string> = {};
            data.forEach((s) => {
                settingsMap[s.key] = s.value;
            });

            applySettingsMap(settingsMap);

            // Load banks (always load to ensure availability)
            try {
                bankAccounts = await api.payment.listBanks();
            } catch (e) {
                console.error("Failed to load banks:", e);
            }

            superadminPlatformSettingsCache.set({
                settingsMap,
                bankAccounts,
                fetchedAt: Date.now(),
            });
        } catch (err) {
            console.error("Failed to load settings:", err);
            toast.error(
                get(t)("superadmin.settings.errors.load_failed") ||
                    "Failed to load settings",
            );
        } finally {
            if (!opts.silent) loading = false;
        }
    }

    function handleChange() {
        hasChanges = true;
    }

    async function saveSettings() {
        saving = true;
        try {
            const updates = [
                // Maintenance
                api.settings.upsert(
                    "app_public_url",
                    appPublicUrl,
                    "Public Application URL",
                ),
                api.settings.upsert(
                    "currency_code",
                    currencyCode.toUpperCase(),
                    "Default currency code (ISO 4217)",
                ),
                api.settings.upsert(
                    "maintenance_mode",
                    maintenanceMode ? "true" : "false",
                    "Global maintenance mode",
                ),
                api.settings.upsert(
                    "maintenance_message",
                    maintenanceMessage,
                    "Message shown during maintenance",
                ),
                // Authentication
                api.settings.upsert(
                    "auth_allow_registration",
                    authAllowRegistration ? "true" : "false",
                    "Allow public registration",
                ),
                api.settings.upsert(
                    "auth_require_email_verification",
                    authRequireEmailVerification ? "true" : "false",
                    "Require email verification before login",
                ),
                api.settings.upsert(
                    "auth_jwt_expiry_hours",
                    authJwtExpiryHours.toString(),
                    "JWT token expiry in hours",
                ),
                api.settings.upsert(
                    "auth_session_timeout_minutes",
                    authSessionTimeoutMinutes.toString(),
                    "Inactivity session timeout in minutes",
                ),
                // Password Policy
                api.settings.upsert(
                    "auth_password_min_length",
                    authPasswordMinLength.toString(),
                    "Minimum password length",
                ),
                api.settings.upsert(
                    "auth_password_require_uppercase",
                    authPasswordRequireUppercase ? "true" : "false",
                    "Require uppercase letter in password",
                ),
                api.settings.upsert(
                    "auth_password_require_number",
                    authPasswordRequireNumber ? "true" : "false",
                    "Require number in password",
                ),
                api.settings.upsert(
                    "auth_password_require_special",
                    authPasswordRequireSpecial ? "true" : "false",
                    "Require special character in password",
                ),
                api.settings.upsert(
                    "auth_logout_all_on_password_change",
                    authLogoutAllOnPasswordChange ? "true" : "false",
                    "Logout all sessions on password change",
                ),
                // Security
                api.settings.upsert(
                    "auth_max_login_attempts",
                    maxLoginAttempts.toString(),
                    "Maximum failed login attempts",
                ),
                api.settings.upsert(
                    "auth_lockout_duration_minutes",
                    lockoutDurationMinutes.toString(),
                    "Lockout duration in minutes",
                ),
                api.settings.upsert(
                    "api_rate_limit_per_minute",
                    apiRateLimitPerMinute.toString(),
                    "API rate limit per minute",
                ),
                api.settings.upsert(
                    "enable_ip_blocking",
                    enableIpBlocking ? "true" : "false",
                    "Enable IP blocking",
                ),
                // 2FA Configuration
                api.settings.upsert(
                    "2fa_enabled",
                    twoFAEnabled ? "true" : "false",
                    "Enable Two-Factor Authentication",
                ),
                api.settings.upsert(
                    "2fa_methods",
                    [
                        twoFAMethodTotp ? "totp" : "",
                        twoFAMethodEmail ? "email" : "",
                    ]
                        .filter(Boolean)
                        .join(",") || "totp",
                    "Available 2FA methods",
                ),
                api.settings.upsert(
                    "2fa_email_otp_expiry_minutes",
                    twoFAEmailOtpExpiryMinutes.toString(),
                    "Email OTP expiry in minutes",
                ),
                // Storage
                api.settings.upsert(
                    "storage_max_file_size_mb",
                    storageMaxFileSizeMb.toString(),
                    "Maximum file upload size in MB",
                ),
                api.settings.upsert(
                    "storage_allowed_extensions",
                    storageAllowedExtensions,
                    "Allowed file extensions",
                ),
                // Storage Driver
                api.settings.upsert(
                    "storage_driver",
                    storageDriver,
                    "Storage Driver",
                ),
                api.settings.upsert(
                    "storage_s3_bucket",
                    storageS3Bucket,
                    "S3 Bucket",
                ),
                api.settings.upsert(
                    "storage_s3_region",
                    storageS3Region,
                    "S3 Region",
                ),
                api.settings.upsert(
                    "storage_s3_endpoint",
                    storageS3Endpoint,
                    "S3 Endpoint",
                ),
                api.settings.upsert(
                    "storage_s3_access_key",
                    storageS3AccessKey,
                    "S3 Access Key",
                ),
                api.settings.upsert(
                    "storage_s3_secret_key",
                    storageS3SecretKey,
                    "S3 Secret Key",
                ),
                api.settings.upsert(
                    "storage_s3_public_url",
                    storageS3PublicUrl,
                    "S3 Public URL",
                ),
                // Payment
                api.settings.upsert(
                    "payment_midtrans_enabled",
                    paymentMidtransEnabled ? "true" : "false",
                    "Enable Midtrans",
                ),
                api.settings.upsert(
                    "payment_midtrans_merchant_id",
                    paymentMidtransMerchantId,
                    "Midtrans Merchant ID",
                ),
                api.settings.upsert(
                    "payment_midtrans_server_key",
                    paymentMidtransServerKey,
                    "Midtrans Server Key",
                ),
                api.settings.upsert(
                    "payment_midtrans_client_key",
                    paymentMidtransClientKey,
                    "Midtrans Client Key",
                ),
                api.settings.upsert(
                    "payment_midtrans_is_production",
                    paymentMidtransIsProduction ? "true" : "false",
                    "Midtrans Production Mode",
                ),
                api.settings.upsert(
                    "payment_manual_enabled",
                    paymentManualEnabled ? "true" : "false",
                    "Enable Manual Payment",
                ),
                api.settings.upsert(
                    "payment_manual_instructions",
                    paymentManualInstructions,
                    "Manual Payment Instructions",
                ),
            ];

            await Promise.all(updates);

            toast.success(
                get(t)("superadmin.settings.toasts.saved") ||
                    "Settings saved successfully",
            );
            hasChanges = false;

            // Refresh global settings store so currency/locale update immediately
            await appSettings.refresh();

            // Refresh cache with latest values
            const settingsMap: Record<string, string> = {
                app_public_url: appPublicUrl,
                currency_code: currencyCode.toUpperCase(),
                maintenance_mode: maintenanceMode ? "true" : "false",
                maintenance_message: maintenanceMessage,
                auth_allow_registration: authAllowRegistration ? "true" : "false",
                auth_require_email_verification: authRequireEmailVerification
                    ? "true"
                    : "false",
                auth_jwt_expiry_hours: authJwtExpiryHours.toString(),
                auth_session_timeout_minutes: authSessionTimeoutMinutes.toString(),
                auth_password_min_length: authPasswordMinLength.toString(),
                auth_password_require_uppercase: authPasswordRequireUppercase
                    ? "true"
                    : "false",
                auth_password_require_number: authPasswordRequireNumber
                    ? "true"
                    : "false",
                auth_password_require_special: authPasswordRequireSpecial
                    ? "true"
                    : "false",
                auth_logout_all_on_password_change: authLogoutAllOnPasswordChange
                    ? "true"
                    : "false",
                auth_max_login_attempts: maxLoginAttempts.toString(),
                auth_lockout_duration_minutes: lockoutDurationMinutes.toString(),
                api_rate_limit_per_minute: apiRateLimitPerMinute.toString(),
                enable_ip_blocking: enableIpBlocking ? "true" : "false",
                "2fa_enabled": twoFAEnabled ? "true" : "false",
                "2fa_methods": [
                    twoFAMethodTotp ? "totp" : null,
                    twoFAMethodEmail ? "email" : null,
                ]
                    .filter(Boolean)
                    .join(","),
                "2fa_email_otp_expiry_minutes":
                    twoFAEmailOtpExpiryMinutes.toString(),
                storage_max_file_size_mb: storageMaxFileSizeMb.toString(),
                storage_allowed_extensions: storageAllowedExtensions,
                storage_driver: storageDriver,
                storage_s3_bucket: storageS3Bucket,
                storage_s3_region: storageS3Region,
                storage_s3_endpoint: storageS3Endpoint,
                storage_s3_access_key: storageS3AccessKey,
                storage_s3_secret_key: storageS3SecretKey,
                storage_s3_public_url: storageS3PublicUrl,
                payment_midtrans_enabled: paymentMidtransEnabled ? "true" : "false",
                payment_midtrans_merchant_id: paymentMidtransMerchantId,
                payment_midtrans_server_key: paymentMidtransServerKey,
                payment_midtrans_client_key: paymentMidtransClientKey,
                payment_midtrans_is_production: paymentMidtransIsProduction
                    ? "true"
                    : "false",
                payment_manual_enabled: paymentManualEnabled ? "true" : "false",
                payment_manual_instructions: paymentManualInstructions,
            };

            superadminPlatformSettingsCache.set({
                settingsMap,
                bankAccounts,
                fetchedAt: Date.now(),
            });
        } catch (err) {
            console.error("Failed to save settings:", err);
            toast.error(
                get(t)("superadmin.settings.errors.save_failed") ||
                    "Failed to save settings",
            );
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
                get(t)("superadmin.settings.errors.missing_test_email") ||
                    "Please enter an email address",
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
                (get(t)("superadmin.settings.errors.test_email_failed") ||
                    "Failed to send test email: ") + String(error),
            );
        } finally {
            sendingTestEmail = false;
        }
    }

    async function addBank() {
        if (!newBankName || !newAccountNumber || !newAccountHolder) return;
        addingBank = true;
        try {
            await api.payment.createBank(
                newBankName,
                newAccountNumber,
                newAccountHolder,
            );
            bankAccounts = await api.payment.listBanks();
            newBankName = "";
            newAccountNumber = "";
            newAccountHolder = "";
            toast.success(
                get(t)("superadmin.settings.toasts.bank_added") ||
                    "Bank account added",
            );
        } catch (e: any) {
            toast.error(
                e.message ||
                    get(t)("superadmin.settings.errors.bank_add_failed") ||
                    "Failed to add bank",
            );
        } finally {
            addingBank = false;
        }
    }

    async function deleteBank(id: string) {
        if (
            !confirm(
                get(t)("superadmin.settings.confirm.are_you_sure") ||
                    "Are you sure?",
            )
        )
            return;
        try {
            await api.payment.deleteBank(id);
            bankAccounts = bankAccounts.filter((b) => b.id !== id);
            toast.success(
                get(t)("superadmin.settings.toasts.bank_removed") ||
                    "Bank account removed",
            );
        } catch (e: any) {
            toast.error(
                e.message ||
                    get(t)("superadmin.settings.errors.bank_remove_failed") ||
                    "Failed to delete bank",
            );
        }
    }

    function showMessage(type: "success" | "error", msg: string) {
        if (type === "success") toast.success(msg);
        else toast.error(msg);
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
                        {$t("superadmin.settings.loading") ||
                            "Loading settings..."}
                    </p>
                </div>
            {:else}
                <!-- General & Maintenance Tab -->
                {#if activeTab === "general"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>
                                {$t("superadmin.settings.sections.general") ||
                                    "General Settings"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info full-width">
                                    <label
                                        class="setting-label"
                                        for="public-url"
                                        >{$t(
                                            "superadmin.settings.fields.public_url",
                                        ) || "Public Application URL"}</label
                                    >
                                    <p class="setting-description">
                                        Base URL for redirects, emails, and
                                        payment callbacks (e.g.
                                        https://app.example.com).
                                    </p>
                                    <input
                                        type="text"
                                        id="public-url"
                                        bind:value={appPublicUrl}
                                        on:input={handleChange}
                                        class="form-input"
                                        placeholder={$t(
                                            "superadmin.settings.placeholders.url",
                                        ) || "https://..."}
                                    />
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="currency-code"
                                        >{$t(
                                            "superadmin.settings.fields.currency",
                                        ) || "Default Currency"}</label
                                    >
                                    <p class="setting-description">
                                        Currency used for plan pricing and invoice
                                        display (ISO 4217, e.g. IDR, USD).
                                    </p>
                                </div>
                                <select
                                    id="currency-code"
                                    class="form-input"
                                    bind:value={currencyCode}
                                    on:change={handleChange}
                                >
                                    {#each currencyCodeOptions as opt}
                                        <option value={opt}>{opt}</option>
                                    {/each}
                                </select>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="maintenance-mode"
                                        >{$t(
                                            "superadmin.settings.fields.maintenance_mode",
                                        ) || "Enable Maintenance Mode"}</label
                                    >
                                    <p class="setting-description">
                                        When enabled, all users except
                                        superadmins will see a maintenance page.
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        id="maintenance-mode"
                                        bind:checked={maintenanceMode}
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info full-width">
                                    <label
                                        class="setting-label"
                                        for="maintenance-message"
                                        >{$t(
                                            "superadmin.settings.fields.maintenance_message",
                                        ) || "Maintenance Message"}</label
                                    >
                                    <p class="setting-description">
                                        Message displayed to users during
                                        maintenance.
                                    </p>
                                    <textarea
                                        id="maintenance-message"
                                        bind:value={maintenanceMessage}
                                        on:input={handleChange}
                                        rows="3"
                                        placeholder={$t(
                                            "superadmin.settings.placeholders.maintenance_message",
                                        ) || "Enter maintenance message..."}
                                    ></textarea>
                                </div>
                            </div>
                        </div>
                    </div>
                {/if}

                <!-- Authentication Tab -->
                {#if activeTab === "auth"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>
                                {$t("superadmin.settings.sections.auth") ||
                                    "Authentication Policy"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="allow-registration"
                                        >{$t(
                                            "superadmin.settings.auth.allow_public_registration.label",
                                        ) || "Allow Public Registration"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.auth.allow_public_registration.desc",
                                        ) || "Allow new users to sign up freely."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        id="allow-registration"
                                        bind:checked={authAllowRegistration}
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="require-email-verify"
                                        >{$t(
                                            "superadmin.settings.auth.require_email_verification.label",
                                        ) || "Require Email Verification"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.auth.require_email_verification.desc",
                                        ) || "Users must verify email before logging in."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        id="require-email-verify"
                                        bind:checked={
                                            authRequireEmailVerification
                                        }
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="jwt-expiry"
                                        >{$t("superadmin.settings.auth.jwt_expiry.label") ||
                                            "JWT Expiry (Hours)"}</label
                                    >
                                    <p class="setting-description">
                                        {$t("superadmin.settings.auth.jwt_expiry.desc") ||
                                            "How long an auth token remains valid."}
                                    </p>
                                </div>
                                <div class="input-group">
                                    <input
                                        type="number"
                                        id="jwt-expiry"
                                        bind:value={authJwtExpiryHours}
                                        on:input={handleChange}
                                        min="1"
                                    />
                                    <span class="input-suffix"
                                        >{$t("common.units.hours") || "hours"}</span
                                    >
                                </div>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="session-timeout"
                                        >{$t(
                                            "superadmin.settings.auth.session_timeout.label",
                                        ) || "Session Timeout (Minutes)"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.auth.session_timeout.desc",
                                        ) || "Auto-logout after inactivity."}
                                    </p>
                                </div>
                                <div class="input-group">
                                    <input
                                        type="number"
                                        id="session-timeout"
                                        bind:value={authSessionTimeoutMinutes}
                                        on:input={handleChange}
                                        min="5"
                                    />
                                    <span class="input-suffix"
                                        >{$t("common.units.min") || "min"}</span
                                    >
                                </div>
                            </div>
                        </div>
                    </div>
                {/if}

                <!-- Password Policy Tab -->
                {#if activeTab === "password"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>
                                {$t("superadmin.settings.sections.password") ||
                                    "Password Policy"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="min-pwd-length"
                                        >{$t("superadmin.settings.password.min_length.label") ||
                                            "Minimum Length"}</label
                                    >
                                </div>
                                <div class="input-group">
                                    <input
                                        type="number"
                                        id="min-pwd-length"
                                        bind:value={authPasswordMinLength}
                                        on:input={handleChange}
                                        min="6"
                                    />
                                    <span class="input-suffix"
                                        >{$t("common.units.chars") || "chars"}</span
                                    >
                                </div>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="require-uppercase"
                                        >{$t(
                                            "superadmin.settings.password.require_uppercase.label",
                                        ) || "Require Uppercase"}</label
                                    >
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        id="require-uppercase"
                                        bind:checked={
                                            authPasswordRequireUppercase
                                        }
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="require-number"
                                        >{$t(
                                            "superadmin.settings.password.require_number.label",
                                        ) || "Require Number"}</label
                                    >
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        id="require-number"
                                        bind:checked={authPasswordRequireNumber}
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="require-special"
                                        >{$t(
                                            "superadmin.settings.password.require_special.label",
                                        ) || "Require Special Character"}</label
                                    >
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        id="require-special"
                                        bind:checked={
                                            authPasswordRequireSpecial
                                        }
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                        </div>
                    </div>
                {/if}

                <!-- Security Tab -->
                {#if activeTab === "security"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>
                                {$t("superadmin.settings.sections.security") ||
                                    "Security & Rate Limiting"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="max-login-attempts"
                                        >{$t(
                                            "superadmin.settings.security.max_login_attempts.label",
                                        ) || "Max Login Attempts"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.security.max_login_attempts.desc",
                                        ) ||
                                            "Number of failed login attempts before account lockout."}
                                    </p>
                                </div>
                                <div class="input-group">
                                    <input
                                        type="number"
                                        id="max-login-attempts"
                                        bind:value={maxLoginAttempts}
                                        on:input={handleChange}
                                        min="1"
                                        max="20"
                                    />
                                    <span class="input-suffix"
                                        >{$t("common.units.attempts") ||
                                            "attempts"}</span
                                    >
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="lockout-duration"
                                        >{$t(
                                            "superadmin.settings.security.lockout_duration.label",
                                        ) || "Lockout Duration"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.security.lockout_duration.desc",
                                        ) ||
                                            "How long a user stays locked out after max failed attempts."}
                                    </p>
                                </div>
                                <div class="input-group">
                                    <input
                                        type="number"
                                        id="lockout-duration"
                                        bind:value={lockoutDurationMinutes}
                                        on:input={handleChange}
                                        min="1"
                                        max="1440"
                                    />
                                    <span class="input-suffix"
                                        >{$t("common.units.minutes") ||
                                            "minutes"}</span
                                    >
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="api-rate-limit"
                                        >{$t(
                                            "superadmin.settings.security.api_rate_limit.label",
                                        ) || "API Rate Limit"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.security.api_rate_limit.desc",
                                        ) || "Maximum API requests allowed per minute per user."}
                                    </p>
                                </div>
                                <div class="input-group">
                                    <input
                                        type="number"
                                        id="api-rate-limit"
                                        bind:value={apiRateLimitPerMinute}
                                        on:input={handleChange}
                                        min="10"
                                        max="1000"
                                    />
                                    <span class="input-suffix"
                                        >{$t("common.units.req_per_min") ||
                                            "req/min"}</span
                                    >
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label"
                                        >{$t(
                                            "superadmin.settings.security.ip_blocking.label",
                                        ) || "Enable IP Blocking"}</span
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.security.ip_blocking.desc",
                                        ) ||
                                            "Automatically block IP addresses with suspicious activity."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={enableIpBlocking}
                                        on:change={handleChange}
                                        aria-label={$t(
                                            "superadmin.settings.security.ip_blocking.aria",
                                        ) || "Enable IP Blocking"}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                        </div>
                    </div>

                    <!-- 2FA Settings Card -->
                    <div
                        class="card section fade-in"
                        style="margin-top: 1.5rem;"
                    >
                        <div class="card-header">
                            <h3>
                                {$t("superadmin.settings.sections.twofa") ||
                                    "Two-Factor Authentication"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label"
                                        >{$t(
                                            "superadmin.settings.twofa.enable_2fa.label",
                                        ) || "Enable 2FA"}</span
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.twofa.enable_2fa.desc",
                                        ) ||
                                            "Allow users to set up two-factor authentication for enhanced security."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={twoFAEnabled}
                                        on:change={handleChange}
                                        aria-label={$t(
                                            "superadmin.settings.twofa.enable_2fa.aria",
                                        ) || "Enable 2FA"}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label"
                                        >{$t(
                                            "superadmin.settings.twofa.enable_totp.label",
                                        ) || "TOTP (Authenticator App)"}</span
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.twofa.enable_totp.desc",
                                        ) ||
                                            "Allow users to verify with Google Authenticator, Authy, etc."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={twoFAMethodTotp}
                                        on:change={handleChange}
                                        aria-label={$t(
                                            "superadmin.settings.twofa.enable_totp.aria",
                                        ) || "Enable TOTP"}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label"
                                        >{$t(
                                            "superadmin.settings.twofa.email_otp.label",
                                        ) || "Email OTP"}</span
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.twofa.email_otp.desc",
                                        ) ||
                                            "Allow users to receive verification codes via email."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={twoFAMethodEmail}
                                        on:change={handleChange}
                                        aria-label={$t(
                                            "superadmin.settings.twofa.enable_email_otp.aria",
                                        ) || "Enable Email OTP"}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            {#if twoFAMethodEmail}
                                <div class="setting-row">
                                    <div class="setting-info">
                                        <label
                                            class="setting-label"
                                            for="email-otp-expiry"
                                            >{$t(
                                                "superadmin.settings.twofa.email_otp_expiry.label",
                                            ) || "Email OTP Expiry"}</label
                                        >
                                        <p class="setting-description">
                                            {$t(
                                                "superadmin.settings.twofa.email_otp_expiry.desc",
                                            ) ||
                                                "How long email verification codes remain valid."}
                                        </p>
                                    </div>
                                    <div class="input-group">
                                        <input
                                            type="number"
                                            id="email-otp-expiry"
                                            bind:value={
                                                twoFAEmailOtpExpiryMinutes
                                            }
                                            on:input={handleChange}
                                            min="1"
                                            max="30"
                                        />
                                        <span class="input-suffix"
                                            >{$t("common.units.min") ||
                                                "min"}</span
                                        >
                                    </div>
                                </div>
                            {/if}
                        </div>
                    </div>
                {/if}

                <!-- Storage Tab -->
                {#if activeTab === "storage"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>
                                {$t("superadmin.settings.sections.storage") ||
                                    "Storage Configuration"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <!-- Driver Selection -->
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="storage-driver"
                                        >{$t(
                                            "superadmin.settings.storage.driver.label",
                                        ) || "Storage Driver"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.storage.driver.desc",
                                        ) ||
                                            "Choose where files are stored. Local uses the server's disk."}
                                    </p>
                                </div>
                                <div class="input-group">
                                    <select
                                        id="storage-driver"
                                        bind:value={storageDriver}
                                        on:change={handleChange}
                                        class="native-select"
                                    >
                                        <option value="local">
                                            {$t(
                                                "superadmin.settings.storage.driver.options.local",
                                            ) || "Local Disk"}
                                        </option
                                        >
                                        <option value="s3">
                                            {$t(
                                                "superadmin.settings.storage.driver.options.s3",
                                            ) || "AWS S3 / MinIO"}
                                        </option
                                        >
                                        <option value="r2">
                                            {$t(
                                                "superadmin.settings.storage.driver.options.r2",
                                            ) || "Cloudflare R2"}
                                        </option
                                        >
                                    </select>
                                </div>
                            </div>

                            {#if storageDriver !== "local"}
                                <div class="sub-settings fade-in">
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="bucket-name"
                                                >{$t(
                                                    "superadmin.settings.storage.s3.bucket_name",
                                                ) || "Bucket Name"}</label
                                            >
                                        </div>
                                        <input
                                            type="text"
                                            id="bucket-name"
                                            bind:value={storageS3Bucket}
                                            on:input={handleChange}
                                            placeholder={$t(
                                                "superadmin.settings.placeholders.s3_bucket",
                                            ) || "e.g. my-app-uploads"}
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="bucket-region"
                                                >{$t(
                                                    "superadmin.settings.storage.s3.region",
                                                ) || "Region"}</label
                                            >
                                            <p class="setting-description">
                                                {$t(
                                                    "superadmin.settings.storage.s3.region_hint",
                                                ) || "Use 'auto' for R2."}
                                            </p>
                                        </div>
                                        <input
                                            type="text"
                                            id="bucket-region"
                                            bind:value={storageS3Region}
                                            on:input={handleChange}
                                            placeholder={$t(
                                                "superadmin.settings.placeholders.s3_region",
                                            ) || "us-east-1"}
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="bucket-endpoint"
                                                >{$t(
                                                    "superadmin.settings.storage.s3.endpoint_url",
                                                ) || "Endpoint URL"}</label
                                            >
                                            <p class="setting-description">
                                                {$t(
                                                    "superadmin.settings.storage.s3.endpoint_hint",
                                                ) ||
                                                    "Required for R2 (https://ID.r2.cloudflarestorage.com) or MinIO."}
                                            </p>
                                        </div>
                                        <input
                                            type="text"
                                            id="bucket-endpoint"
                                            bind:value={storageS3Endpoint}
                                            on:input={handleChange}
                                            placeholder={$t(
                                                "superadmin.settings.placeholders.url",
                                            ) || "https://..."}
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="access-key"
                                                >{$t(
                                                    "superadmin.settings.storage.s3.access_key_id",
                                                ) || "Access Key ID"}</label
                                            >
                                        </div>
                                        <input
                                            type="text"
                                            id="access-key"
                                            bind:value={storageS3AccessKey}
                                            on:input={handleChange}
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="access-secret-key"
                                                >{$t(
                                                    "superadmin.settings.storage.s3.secret_access_key",
                                                ) || "Secret Access Key"}</label
                                            >
                                        </div>
                                        <input
                                            type="password"
                                            id="access-secret-key"
                                            bind:value={storageS3SecretKey}
                                            on:input={handleChange}
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="public-access-url"
                                                >{$t(
                                                    "superadmin.settings.storage.s3.public_url_optional",
                                                ) ||
                                                    "Public Access URL (Optional)"}</label
                                            >
                                            <p class="setting-description">
                                                {$t(
                                                    "superadmin.settings.storage.s3.public_url_hint",
                                                ) || "CDN URL if serving files publicly."}
                                            </p>
                                        </div>
                                        <input
                                            type="text"
                                            id="public-access-url"
                                            bind:value={storageS3PublicUrl}
                                            on:input={handleChange}
                                            placeholder={$t(
                                                "superadmin.settings.placeholders.public_url",
                                            ) || "https://cdn.example.com"}
                                        />
                                    </div>
                                </div>
                            {/if}

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="max-file-size"
                                        >{$t(
                                            "superadmin.settings.storage.max_file_size_mb",
                                        ) || "Max File Size (MB)"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.storage.max_file_size_mb_desc",
                                        ) ||
                                            "Maximum allowed size for a single file upload."}
                                    </p>
                                </div>
                                <div class="input-group">
                                    <input
                                        type="number"
                                        id="max-file-size"
                                        bind:value={storageMaxFileSizeMb}
                                        on:input={handleChange}
                                        min="1"
                                    />
                                    <span class="input-suffix"
                                        >{$t("common.units.mb") || "MB"}</span
                                    >
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info full-width">
                                    <label
                                        class="setting-label"
                                        for="allowed-extensions"
                                        >{$t(
                                            "superadmin.settings.storage.allowed_extensions",
                                        ) || "Allowed Extensions"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.storage.allowed_extensions_desc",
                                        ) ||
                                            "Comma-separated list of allowed file extensions (e.g., jpg, png, pdf). Use * for all."}
                                    </p>
                                    <textarea
                                        id="allowed-extensions"
                                        bind:value={storageAllowedExtensions}
                                        on:input={handleChange}
                                        rows="3"
                                        placeholder={$t(
                                            "superadmin.settings.placeholders.extensions",
                                        ) || "jpg, png, pdf..."}
                                    ></textarea>
                                </div>
                            </div>
                        </div>
                    </div>
                {/if}

                <!-- Payment Tab -->
                {#if activeTab === "payment"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>
                                {$t("superadmin.settings.sections.payment") ||
                                    "Payment Gateway (Midtrans)"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="midtrans-gateway-toggle"
                                        >{$t(
                                            "superadmin.settings.payment.midtrans.enable.label",
                                        ) || "Enable Midtrans Gateway"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.payment.midtrans.enable.desc",
                                        ) || "Allow users to pay online via Midtrans."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        id="midtrans-gateway-toggle"
                                        type="checkbox"
                                        bind:checked={paymentMidtransEnabled}
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            {#if paymentMidtransEnabled}
                                <div class="sub-settings fade-in">
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="midtrans-merchant-id"
                                                >{$t(
                                                    "superadmin.settings.payment.midtrans.merchant_id",
                                                ) || "Merchant ID"}</label
                                            >
                                        </div>
                                        <input
                                            id="midtrans-merchant-id"
                                            type="text"
                                            bind:value={
                                                paymentMidtransMerchantId
                                            }
                                            on:input={handleChange}
                                            class="form-input"
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="midtrans-server-key"
                                                >{$t(
                                                    "superadmin.settings.payment.midtrans.server_key",
                                                ) || "Server Key"}</label
                                            >
                                            <p class="setting-description">
                                                {$t(
                                                    "superadmin.settings.payment.midtrans.server_key_hint",
                                                ) ||
                                                    "From Midtrans Dashboard > Settings > Access Keys."}
                                            </p>
                                        </div>
                                        <input
                                            id="midtrans-server-key"
                                            type="password"
                                            bind:value={
                                                paymentMidtransServerKey
                                            }
                                            on:input={handleChange}
                                            class="form-input"
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="midtrans-client-key"
                                                >{$t(
                                                    "superadmin.settings.payment.midtrans.client_key",
                                                ) || "Client Key"}</label
                                            >
                                            <p class="setting-description">
                                                {$t(
                                                    "superadmin.settings.payment.midtrans.client_key_hint",
                                                ) || "Public key for frontend Snap.js."}
                                            </p>
                                        </div>
                                        <input
                                            id="midtrans-client-key"
                                            type="text"
                                            bind:value={
                                                paymentMidtransClientKey
                                            }
                                            on:input={handleChange}
                                            class="form-input"
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="midtrans-production"
                                                >{$t(
                                                    "superadmin.settings.payment.midtrans.production_mode",
                                                ) || "Production Mode"}</label
                                            >
                                            <p class="setting-description">
                                                {$t(
                                                    "superadmin.settings.payment.midtrans.production_mode_desc",
                                                ) ||
                                                    "Enable for real transactions. Disable for Sandbox."}
                                            </p>
                                        </div>
                                        <label class="toggle">
                                            <input
                                                id="midtrans-production"
                                                type="checkbox"
                                                bind:checked={
                                                    paymentMidtransIsProduction
                                                }
                                                on:change={handleChange}
                                            />
                                            <span class="slider"></span>
                                        </label>
                                    </div>
                                </div>
                            {/if}
                        </div>
                    </div>

                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>
                                {$t(
                                    "superadmin.settings.sections.manual_bank_transfer",
                                ) || "Manual Bank Transfer"}
                            </h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="manual-transfer-toggle"
                                        >{$t(
                                            "superadmin.settings.payment.manual.enable.label",
                                        ) || "Enable Manual Transfer"}</label
                                    >
                                    <p class="setting-description">
                                        {$t(
                                            "superadmin.settings.payment.manual.enable.desc",
                                        ) ||
                                            "Allow users to pay via bank transfer and upload proof."}
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        id="manual-transfer-toggle"
                                        type="checkbox"
                                        bind:checked={paymentManualEnabled}
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            {#if paymentManualEnabled}
                                <div class="sub-settings fade-in">
                                    <div class="setting-row">
                                        <div class="setting-info full-width">
                                            <label
                                                class="setting-label"
                                                for="manual-instructions"
                                                >{$t(
                                                    "superadmin.settings.payment.manual.instructions_label",
                                                ) || "Payment Instructions"}</label
                                            >
                                            <p class="setting-description">
                                                {$t(
                                                    "superadmin.settings.payment.manual.instructions_desc",
                                                ) ||
                                                    "Instructions shown to user when they select Manual Transfer."}
                                            </p>
                                            <textarea
                                                id="manual-instructions"
                                                bind:value={
                                                    paymentManualInstructions
                                                }
                                                on:input={handleChange}
                                                rows="3"
                                                placeholder={$t(
                                                    "superadmin.settings.placeholders.manual_instructions",
                                                ) ||
                                                    "Please transfer to one of the bank accounts below and upload proof."}
                                            ></textarea>
                                        </div>
                                    </div>

                                    <div class="bank-accounts-list">
                                        <h4 class="subsection-title">
                                            Bank Accounts
                                        </h4>
                                        {#if bankAccounts.length > 0}
                                            {#if isMobile}
                                                <div class="bank-cards">
                                                    {#each bankAccounts as bank}
                                                        <div class="bank-card">
                                                            <div class="bank-card-top">
                                                                <div>
                                                                    <div class="bank-name">
                                                                        {bank.bank_name}
                                                                    </div>
                                                                    <div class="bank-sub">
                                                                        {bank.account_holder}
                                                                    </div>
                                                                </div>
                                                                <button
                                                                    class="btn-icon danger"
                                                                    type="button"
                                                                    title={$t(
                                                                        "superadmin.settings.actions.remove",
                                                                    ) ||
                                                                        "Remove"}
                                                                    on:click={() =>
                                                                        deleteBank(
                                                                            bank.id,
                                                                        )}
                                                                >
                                                                    <Icon
                                                                        name="trash"
                                                                        size={16}
                                                                    />
                                                                </button>
                                                            </div>
                                                            <div class="bank-number mono">
                                                                {bank.account_number}
                                                            </div>
                                                        </div>
                                                    {/each}
                                                </div>
                                            {:else}
                                                <table class="simple-table">
                                                    <thead>
                                                        <tr>
                                                            <th>
                                                                {$t(
                                                                    "superadmin.settings.bank.table.bank",
                                                                ) || "Bank"}
                                                            </th>
                                                            <th>
                                                                {$t(
                                                                    "superadmin.settings.bank.table.number",
                                                                ) || "Number"}
                                                            </th>
                                                            <th>
                                                                {$t(
                                                                    "superadmin.settings.bank.table.holder",
                                                                ) || "Holder"}
                                                            </th>
                                                            <th>
                                                                {$t(
                                                                    "superadmin.settings.bank.table.action",
                                                                ) || "Action"}
                                                            </th>
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {#each bankAccounts as bank}
                                                            <tr>
                                                                <td
                                                                    >{bank.bank_name}</td
                                                                >
                                                                <td
                                                                    >{bank.account_number}</td
                                                                >
                                                                <td
                                                                    >{bank.account_holder}</td
                                                                >
                                                                <td>
                                                                    <button
                                                                        class="btn-icon danger"
                                                                        type="button"
                                                                        on:click={() =>
                                                                            deleteBank(
                                                                                bank.id,
                                                                            )}
                                                                    >
                                                                        <Icon
                                                                            name="trash"
                                                                            size={16}
                                                                        />
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        {/each}
                                                    </tbody>
                                                </table>
                                            {/if}
                                        {:else}
                                            <p class="text-muted">
                                                {$t(
                                                    "superadmin.settings.bank.empty",
                                                ) || "No bank accounts added yet."}
                                            </p>
                                        {/if}
                                    </div>

                                    <div class="add-bank-form">
                                        <h4>
                                            {$t(
                                                "superadmin.settings.bank.add_new_account",
                                            ) || "Add New Account"}
                                        </h4>
                                        <div class="form-row-inline">
                                            <input
                                                type="text"
                                                bind:value={newBankName}
                                                placeholder={$t(
                                                    "superadmin.settings.placeholders.bank_name",
                                                ) ||
                                                    "Bank Name (e.g. BCA)"}
                                                class="form-input"
                                            />
                                            <input
                                                type="text"
                                                bind:value={newAccountNumber}
                                                placeholder={$t(
                                                    "superadmin.settings.placeholders.bank_account_number",
                                                ) || "Account Number"}
                                                class="form-input"
                                            />
                                            <input
                                                type="text"
                                                bind:value={newAccountHolder}
                                                placeholder={$t(
                                                    "superadmin.settings.placeholders.bank_account_holder",
                                                ) || "Account Holder"}
                                                class="form-input"
                                            />
                                            <button
                                                class="btn btn-primary"
                                                on:click={addBank}
                                                disabled={addingBank}
                                            >
                                                <Icon name="plus" size={16} />
                                                {$t(
                                                    "superadmin.settings.actions.add",
                                                ) || "Add"}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            {/if}
                        </div>
                    </div>
                {/if}

                <!-- Actions Footer -->
                <div class="actions-footer">
                    <button
                        class="btn btn-secondary"
                        disabled={!hasChanges || saving}
                        on:click={discardChanges}
                    >
                        {$t("superadmin.settings.actions.reset") || "Reset"}
                    </button>
                    <button
                        class="btn btn-primary"
                        on:click={saveSettings}
                        disabled={!hasChanges || saving}
                    >
                        {#if saving}
                            <div class="spinner-sm"></div>
                            {$t("superadmin.settings.actions.saving") ||
                                "Saving..."}
                        {:else}
                            <Icon name="save" size={18} />
                            {$t("superadmin.settings.actions.save") ||
                                "Save Changes"}
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

    .card-header {
        padding: 1rem 1.5rem;
        border-bottom: 1px solid var(--border-color);
        background: rgba(0, 0, 0, 0.2);
    }

    .card-header h3 {
        margin: 0;
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .card-body {
        padding: 1.5rem;
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

    .setting-row {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        padding: 1.25rem 0;
        border-bottom: 1px solid var(--border-color);
    }

    .setting-row:last-child {
        border-bottom: none;
    }

    .setting-info {
        flex: 1;
        padding-right: 1.5rem;
    }

    .setting-info.full-width {
        width: 100%;
        padding-right: 0;
    }

    .setting-label {
        font-weight: 600;
        color: var(--text-primary);
        font-size: 0.95rem;
        display: block;
        margin-bottom: 0.25rem;
    }

    .setting-description {
        color: var(--text-secondary);
        font-size: 0.85rem;
        margin: 0;
        line-height: 1.4;
    }

    textarea {
        width: 100%;
        padding: 0.75rem 1rem;
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        color: var(--text-primary);
        font-size: 0.9rem;
        resize: vertical;
        min-height: 80px;
        margin-top: 0.75rem;
    }

    textarea:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 2px var(--color-primary-subtle);
    }

    /* Toggle Switch */
    .toggle {
        position: relative;
        display: inline-block;
        width: 52px;
        height: 28px;
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
        background-color: var(--bg-tertiary);
        transition: 0.3s;
        border-radius: 28px;
    }

    .slider:before {
        position: absolute;
        content: "";
        height: 20px;
        width: 20px;
        left: 4px;
        bottom: 4px;
        background-color: white;
        transition: 0.3s;
        border-radius: 50%;
    }

    input:checked + .slider {
        background-color: var(--color-primary);
    }

    input:checked + .slider:before {
        transform: translateX(24px);
    }

    /* Actions Footer */
    .actions-footer {
        margin-top: 1.5rem;
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
        position: sticky;
        bottom: 0;
        padding: 1rem 0;
        background: linear-gradient(
            to top,
            var(--bg-app) 80%,
            transparent 100%
        );
        z-index: 5;
    }

    .btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.5rem;
        border-radius: var(--radius-sm);
        font-weight: 600;
        font-size: 0.9rem;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover:not(:disabled) {
        background: var(--color-primary-hover);
    }

    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .btn-secondary:hover:not(:disabled) {
        background: var(--bg-hover);
    }

    .btn:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .input-group {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .input-group input[type="number"] {
        width: 100px;
        padding: 0.5rem 0.75rem;
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        color: var(--text-primary);
        font-size: 0.9rem;
        text-align: center;
    }

    .input-group input[type="number"]:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 2px var(--color-primary-subtle);
    }

    .input-suffix {
        color: var(--text-secondary);
        font-size: 0.85rem;
        white-space: nowrap;
    }

    .form-input {
        width: 100%;
        max-width: 400px;
        padding: 0.5rem 0.75rem;
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        color: var(--text-primary);
        font-size: 0.9rem;
    }

    .form-input:focus {
        outline: none;
        border-color: var(--color-primary);
    }

    .native-select {
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        padding: 0.5rem 1rem;
        border-radius: var(--radius-sm);
        color: var(--text-primary);
        font-size: 0.9rem;
        cursor: pointer;
        min-width: 150px;
    }

    .native-select:focus {
        outline: none;
        border-color: var(--color-primary);
    }

    .sub-settings {
        background: rgba(0, 0, 0, 0.1);
        border-radius: var(--radius-md);
        padding: 1rem 1.5rem;
        margin-bottom: 1.5rem;
        border: 1px solid var(--border-color);
    }

    .sub-settings input[type="text"],
    .sub-settings input[type="password"] {
        width: 100%;
        max-width: 400px;
        padding: 0.5rem 0.75rem;
        background: var(--bg-app);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        color: var(--text-primary);
        font-size: 0.9rem;
    }

    .text-muted {
        color: var(--text-secondary);
        font-style: italic;
        font-size: 0.9rem;
    }

    .subsection-title {
        margin: 1.5rem 0 0.5rem 0;
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .simple-table {
        width: 100%;
        border-collapse: collapse;
        margin-bottom: 1.5rem;
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        overflow: hidden;
    }

    .simple-table th,
    .simple-table td {
        padding: 0.75rem 1rem;
        text-align: left;
        border-bottom: 1px solid var(--border-color);
        font-size: 0.9rem;
    }

    .simple-table th {
        background: var(--bg-tertiary);
        font-weight: 600;
        color: var(--text-secondary);
    }

    .form-row-inline {
        display: flex;
        gap: 0.75rem;
        align-items: center;
        flex-wrap: wrap;
    }

    .form-row-inline .form-input {
        flex: 1;
        min-width: 150px;
    }

    .mono {
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
            "Liberation Mono", "Courier New", monospace;
    }

    .bank-cards {
        display: grid;
        grid-template-columns: 1fr;
        gap: 0.75rem;
        margin-bottom: 1rem;
    }

    .bank-card {
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: var(--radius-md);
        padding: 0.9rem;
    }

    .bank-card-top {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 0.75rem;
    }

    .bank-name {
        font-weight: 800;
        color: var(--text-primary);
    }

    .bank-sub {
        margin-top: 0.15rem;
        color: var(--text-secondary);
        font-size: 0.85rem;
        font-weight: 600;
    }

    .bank-number {
        margin-top: 0.75rem;
        padding: 0.55rem 0.75rem;
        border-radius: var(--radius-sm);
        background: rgba(0, 0, 0, 0.12);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        font-weight: 700;
        overflow-wrap: anywhere;
    }

    .btn-icon {
        padding: 0.4rem;
        border-radius: var(--radius-sm);
        background: transparent;
        border: none;
        cursor: pointer;
        color: var(--text-secondary);
        transition: all 0.2s;
    }

    .btn-icon:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .btn-icon.danger:hover {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
    }

    :global([data-theme="light"]) .bank-card {
        background: linear-gradient(135deg, #ffffff, #f7f7fb);
        border-color: rgba(0, 0, 0, 0.06);
        box-shadow:
            0 10px 28px rgba(0, 0, 0, 0.06),
            0 0 0 1px rgba(255, 255, 255, 0.8);
    }

    :global([data-theme="light"]) .bank-number {
        background: rgba(0, 0, 0, 0.03);
        border-color: rgba(0, 0, 0, 0.06);
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

        .card-body {
            padding: 1.1rem;
        }

        .setting-row {
            flex-direction: column;
            align-items: stretch;
            gap: 0.75rem;
        }

        .setting-info {
            padding-right: 0;
        }

        .form-input {
            max-width: none;
        }
    }
</style>

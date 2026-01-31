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

    let loading = true;
    let saving = false;
    let activeTab = "general";

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
            label: "General & Maintenance",
            icon: "settings",
        },
        auth: {
            label: "Authentication",
            icon: "lock",
        },
        password: {
            label: "Password Policy",
            icon: "key",
        },
        security: {
            label: "Security & Rate Limiting",
            icon: "shield",
        },
        storage: {
            label: "Storage Configuration",
            icon: "hard-drive",
        },
        payment: {
            label: "Payment Gateway",
            icon: "credit-card",
        },
    };

    onMount(async () => {
        if (!$isSuperAdmin) {
            goto("/dashboard");
            return;
        }
        await loadSettings();
    });

    async function loadSettings() {
        loading = true;
        try {
            const data = await api.settings.getAll();
            const settingsMap: Record<string, string> = {};
            data.forEach((s) => {
                settingsMap[s.key] = s.value;
            });

            // Maintenance
            maintenanceMode = settingsMap["maintenance_mode"] === "true";
            maintenanceMessage =
                settingsMap["maintenance_message"] ||
                "The system is currently under maintenance. Please try again later.";

            // General
            appPublicUrl =
                settingsMap["app_public_url"] || "http://localhost:3000";
            currencyCode = (settingsMap["currency_code"] || "IDR").toUpperCase();

            // Authentication
            authAllowRegistration =
                settingsMap["auth_allow_registration"] === "true";
            authRequireEmailVerification =
                settingsMap["auth_require_email_verification"] === "true";
            authJwtExpiryHours = parseInt(
                settingsMap["auth_jwt_expiry_hours"] || "24",
            );
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
            paymentMidtransEnabled =
                settingsMap["payment_midtrans_enabled"] === "true";
            paymentMidtransMerchantId =
                settingsMap["payment_midtrans_merchant_id"] || "";
            paymentMidtransServerKey =
                settingsMap["payment_midtrans_server_key"] || "";
            paymentMidtransClientKey =
                settingsMap["payment_midtrans_client_key"] || "";
            paymentMidtransIsProduction =
                settingsMap["payment_midtrans_is_production"] === "true";
            paymentManualEnabled =
                settingsMap["payment_manual_enabled"] !== "false"; // Default true
            paymentManualInstructions =
                settingsMap["payment_manual_instructions"] ||
                "Please transfer to our bank account.";

            // Load banks (always load to ensure availability)
            try {
                bankAccounts = await api.payment.listBanks();
            } catch (e) {
                console.error("Failed to load banks:", e);
            }
        } catch (err) {
            console.error("Failed to load settings:", err);
            toast.error("Failed to load settings");
        } finally {
            loading = false;
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

            toast.success("Settings saved successfully");
            hasChanges = false;

            // Refresh global settings store so currency/locale update immediately
            await appSettings.refresh();
        } catch (err) {
            console.error("Failed to save settings:", err);
            toast.error("Failed to save settings");
        } finally {
            saving = false;
        }
    }

    function discardChanges() {
        loadSettings();
        hasChanges = false;
    }

    async function sendTestEmail() {
        if (!testEmailAddress) {
            toast.error("Please enter an email address");
            return;
        }
        sendingTestEmail = true;
        try {
            const result = await api.settings.sendTestEmail(testEmailAddress);
            toast.success(result);
        } catch (error) {
            console.error(error);
            toast.error("Failed to send test email: " + String(error));
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
            toast.success("Bank account added");
        } catch (e: any) {
            toast.error(e.message || "Failed to add bank");
        } finally {
            addingBank = false;
        }
    }

    async function deleteBank(id: string) {
        if (!confirm("Are you sure?")) return;
        try {
            await api.payment.deleteBank(id);
            bankAccounts = bankAccounts.filter((b) => b.id !== id);
            toast.success("Bank account removed");
        } catch (e: any) {
            toast.error(e.message || "Failed to delete bank");
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
                {#each Object.entries(categories) as [id, cat]}
                    <button
                        class="nav-item {activeTab === id ? 'active' : ''}"
                        on:click={() => {
                            activeTab = id;
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
            items={Object.entries(categories).map(([id, cat]) => ({
                id,
                label: cat.label,
                icon: cat.icon,
            }))}
            bind:activeTab
            title="Platform Settings"
        />

        <main class="content">
            <div class="header-mobile">
                <h1>Platform Settings</h1>
                <p class="subtitle">Global Configuration</p>
            </div>

            {#if loading}
                <div class="loading-state">
                    <div class="spinner"></div>
                    <p>Loading settings...</p>
                </div>
            {:else}
                <!-- General & Maintenance Tab -->
                {#if activeTab === "general"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>General Settings</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info full-width">
                                    <label
                                        class="setting-label"
                                        for="public-url"
                                        >Public Application URL</label
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
                                        placeholder="https://..."
                                    />
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="currency-code"
                                        >Default Currency</label
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
                                        >Enable Maintenance Mode</label
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
                                        >Maintenance Message</label
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
                                        placeholder="Enter maintenance message..."
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
                            <h3>Authentication Policy</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="allow-registration"
                                        >Allow Public Registration</label
                                    >
                                    <p class="setting-description">
                                        Allow new users to sign up freely.
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
                                        >Require Email Verification</label
                                    >
                                    <p class="setting-description">
                                        Users must verify email before logging
                                        in.
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
                                        >JWT Expiry (Hours)</label
                                    >
                                    <p class="setting-description">
                                        How long an auth token remains valid.
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
                                    <span class="input-suffix">hours</span>
                                </div>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="session-timeout"
                                        >Session Timeout (Minutes)</label
                                    >
                                    <p class="setting-description">
                                        Auto-logout after inactivity.
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
                                    <span class="input-suffix">min</span>
                                </div>
                            </div>
                        </div>
                    </div>
                {/if}

                <!-- Password Policy Tab -->
                {#if activeTab === "password"}
                    <div class="card section fade-in">
                        <div class="card-header">
                            <h3>Password Policy</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="min-pwd-length"
                                        >Minimum Length</label
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
                                    <span class="input-suffix">chars</span>
                                </div>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="require-uppercase"
                                        >Require Uppercase</label
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
                                        >Require Number</label
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
                                        >Require Special Character</label
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
                            <h3>Security & Rate Limiting</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="max-login-attempts"
                                        >Max Login Attempts</label
                                    >
                                    <p class="setting-description">
                                        Number of failed login attempts before
                                        account lockout.
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
                                    <span class="input-suffix">attempts</span>
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="lockout-duration"
                                        >Lockout Duration</label
                                    >
                                    <p class="setting-description">
                                        How long a user stays locked out after
                                        max failed attempts.
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
                                    <span class="input-suffix">minutes</span>
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="api-rate-limit"
                                        >API Rate Limit</label
                                    >
                                    <p class="setting-description">
                                        Maximum API requests allowed per minute
                                        per user.
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
                                    <span class="input-suffix">req/min</span>
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label"
                                        >Enable IP Blocking</span
                                    >
                                    <p class="setting-description">
                                        Automatically block IP addresses with
                                        suspicious activity.
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={enableIpBlocking}
                                        on:change={handleChange}
                                        aria-label="Enable IP Blocking"
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
                            <h3>Two-Factor Authentication</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label">Enable 2FA</span
                                    >
                                    <p class="setting-description">
                                        Allow users to set up two-factor
                                        authentication for enhanced security.
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={twoFAEnabled}
                                        on:change={handleChange}
                                        aria-label="Enable 2FA"
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label"
                                        >TOTP (Authenticator App)</span
                                    >
                                    <p class="setting-description">
                                        Allow users to verify with Google
                                        Authenticator, Authy, etc.
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={twoFAMethodTotp}
                                        on:change={handleChange}
                                        aria-label="Enable TOTP"
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info">
                                    <span class="setting-label">Email OTP</span>
                                    <p class="setting-description">
                                        Allow users to receive verification
                                        codes via email.
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={twoFAMethodEmail}
                                        on:change={handleChange}
                                        aria-label="Enable Email OTP"
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
                                            >Email OTP Expiry</label
                                        >
                                        <p class="setting-description">
                                            How long email verification codes
                                            remain valid.
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
                                        <span class="input-suffix">min</span>
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
                            <h3>Storage Configuration</h3>
                        </div>
                        <div class="card-body">
                            <!-- Driver Selection -->
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="storage-driver"
                                        >Storage Driver</label
                                    >
                                    <p class="setting-description">
                                        Choose where files are stored. Local
                                        uses the server's disk.
                                    </p>
                                </div>
                                <div class="input-group">
                                    <select
                                        id="storage-driver"
                                        bind:value={storageDriver}
                                        on:change={handleChange}
                                        class="native-select"
                                    >
                                        <option value="local">Local Disk</option
                                        >
                                        <option value="s3"
                                            >AWS S3 / MinIO</option
                                        >
                                        <option value="r2">Cloudflare R2</option
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
                                                >Bucket Name</label
                                            >
                                        </div>
                                        <input
                                            type="text"
                                            id="bucket-name"
                                            bind:value={storageS3Bucket}
                                            on:input={handleChange}
                                            placeholder="e.g. my-app-uploads"
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="bucket-region"
                                                >Region</label
                                            >
                                            <p class="setting-description">
                                                Use 'auto' for R2.
                                            </p>
                                        </div>
                                        <input
                                            type="text"
                                            id="bucket-region"
                                            bind:value={storageS3Region}
                                            on:input={handleChange}
                                            placeholder="us-east-1"
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="bucket-endpoint"
                                                >Endpoint URL</label
                                            >
                                            <p class="setting-description">
                                                Required for R2
                                                (https://ID.r2.cloudflarestorage.com)
                                                or MinIO.
                                            </p>
                                        </div>
                                        <input
                                            type="text"
                                            id="bucket-endpoint"
                                            bind:value={storageS3Endpoint}
                                            on:input={handleChange}
                                            placeholder="https://..."
                                        />
                                    </div>
                                    <div class="setting-row">
                                        <div class="setting-info">
                                            <label
                                                class="setting-label"
                                                for="access-key"
                                                >Access Key ID</label
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
                                                >Secret Access Key</label
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
                                                >Public Access URL (Optional)</label
                                            >
                                            <p class="setting-description">
                                                CDN URL if serving files
                                                publicly.
                                            </p>
                                        </div>
                                        <input
                                            type="text"
                                            id="public-access-url"
                                            bind:value={storageS3PublicUrl}
                                            on:input={handleChange}
                                            placeholder="https://cdn.example.com"
                                        />
                                    </div>
                                </div>
                            {/if}

                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="max-file-size"
                                        >Max File Size (MB)</label
                                    >
                                    <p class="setting-description">
                                        Maximum allowed size for a single file
                                        upload.
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
                                    <span class="input-suffix">MB</span>
                                </div>
                            </div>

                            <div class="setting-row">
                                <div class="setting-info full-width">
                                    <label
                                        class="setting-label"
                                        for="allowed-extensions"
                                        >Allowed Extensions</label
                                    >
                                    <p class="setting-description">
                                        Comma-separated list of allowed file
                                        extensions (e.g., jpg, png, pdf). Use *
                                        for all.
                                    </p>
                                    <textarea
                                        id="allowed-extensions"
                                        bind:value={storageAllowedExtensions}
                                        on:input={handleChange}
                                        rows="3"
                                        placeholder="jpg, png, pdf..."
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
                            <h3>Payment Gateway (Midtrans)</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="midtrans-gateway-toggle"
                                        >Enable Midtrans Gateway</label
                                    >
                                    <p class="setting-description">
                                        Allow users to pay online via Midtrans.
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
                                                >Merchant ID</label
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
                                                >Server Key</label
                                            >
                                            <p class="setting-description">
                                                From Midtrans Dashboard >
                                                Settings > Access Keys.
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
                                                >Client Key</label
                                            >
                                            <p class="setting-description">
                                                Public key for frontend Snap.js.
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
                                                >Production Mode</label
                                            >
                                            <p class="setting-description">
                                                Enable for real transactions.
                                                Disable for Sandbox.
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
                            <h3>Manual Bank Transfer</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label
                                        class="setting-label"
                                        for="manual-transfer-toggle"
                                        >Enable Manual Transfer</label
                                    >
                                    <p class="setting-description">
                                        Allow users to pay via bank transfer and
                                        upload proof.
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
                                                >Payment Instructions</label
                                            >
                                            <p class="setting-description">
                                                Instructions shown to user when
                                                they select Manual Transfer.
                                            </p>
                                            <textarea
                                                id="manual-instructions"
                                                bind:value={
                                                    paymentManualInstructions
                                                }
                                                on:input={handleChange}
                                                rows="3"
                                                placeholder="Please transfer to one of the bank accounts below and upload proof."
                                            ></textarea>
                                        </div>
                                    </div>

                                    <div class="bank-accounts-list">
                                        <h4 class="subsection-title">
                                            Bank Accounts
                                        </h4>
                                        {#if bankAccounts.length > 0}
                                            <table class="simple-table">
                                                <thead>
                                                    <tr>
                                                        <th>Bank</th>
                                                        <th>Number</th>
                                                        <th>Holder</th>
                                                        <th>Action</th>
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
                                        {:else}
                                            <p class="text-muted">
                                                No bank accounts added yet.
                                            </p>
                                        {/if}
                                    </div>

                                    <div class="add-bank-form">
                                        <h4>Add New Account</h4>
                                        <div class="form-row-inline">
                                            <input
                                                type="text"
                                                bind:value={newBankName}
                                                placeholder="Bank Name (e.g. BCA)"
                                                class="form-input"
                                            />
                                            <input
                                                type="text"
                                                bind:value={newAccountNumber}
                                                placeholder="Account Number"
                                                class="form-input"
                                            />
                                            <input
                                                type="text"
                                                bind:value={newAccountHolder}
                                                placeholder="Account Holder"
                                                class="form-input"
                                            />
                                            <button
                                                class="btn btn-primary"
                                                on:click={addBank}
                                                disabled={addingBank}
                                            >
                                                <Icon name="plus" size={16} /> Add
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
                        Reset
                    </button>
                    <button
                        class="btn btn-primary"
                        on:click={saveSettings}
                        disabled={!hasChanges || saving}
                    >
                        {#if saving}
                            <div class="spinner-sm"></div>
                            Saving...
                        {:else}
                            <Icon name="save" size={18} />
                            Save Changes
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

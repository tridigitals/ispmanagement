<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { user, isAdmin, can, getToken } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { goto } from "$app/navigation";
    import { locale, t, waitLocale } from "svelte-i18n";
    import Icon from "$lib/components/Icon.svelte";
    import MobileFabMenu from "$lib/components/MobileFabMenu.svelte";
    import Input from "$lib/components/Input.svelte";
    import Select from "$lib/components/Select.svelte";
    import type { Setting } from "$lib/api/client";
    import { toast } from "svelte-sonner";

    let loading = $state(true);
    let saving = $state(false);
    let settings = $state<Record<string, Setting>>({});
    let localSettings = $state<Record<string, string>>({});
    let logoBase64 = $state<string | null>(null);
    let activeTab = $state("general");
    let hasChanges = $state(false);

    // Tenant specific state
    let tenantInfo = $state<any>(null);
    let tenantChanges = $state<{ name?: string; customDomain?: string }>({});
    let customDomainAccess = $state(false);

    // Categories configuration
    const categories = {
        general: {
            label: "General",
            icon: "app",
            keys: [
                "app_name",
                "app_description",
                "support_email",
                "default_locale",
                "currency_symbol",
                "app_logo_path",
            ],
        },
        branding: {
            // New Branding & Domain Tab
            label: "Branding & Domain",
            icon: "globe",
            keys: [], // Managed manually
        },
        security: {
            label: "Security",
            icon: "shield",
            keys: [], // Managed manually
        },
        storage: {
            label: "Storage",
            icon: "database",
            keys: [
                "storage_driver",
                "storage_s3_bucket",
                "storage_s3_region",
                "storage_s3_endpoint",
                "storage_s3_access_key",
                "storage_s3_secret_key",
                "storage_s3_public_url",
            ],
        },
        email: {
            label: "Email",
            icon: "mail",
            keys: [
                "email_provider",
                "email_smtp_host",
                "email_smtp_port",
                "email_smtp_username",
                "email_smtp_password",
                "email_smtp_encryption",
                "email_api_key",
                "email_from_address",
                "email_from_name",
                "email_webhook_url",
            ],
        },
        payment: {
            label: "Payments",
            icon: "credit-card",
            keys: [
                "payment_midtrans_enabled",
                "payment_midtrans_merchant_id",
                "payment_midtrans_client_key",
                "payment_midtrans_server_key",
                "payment_midtrans_is_production",
                "payment_manual_enabled",
                "payment_manual_instructions",
                "payment_manual_accounts",
            ],
        },
    };

    onMount(async () => {
        if (!$isAdmin || !$can("read", "settings")) {
            goto("/unauthorized");
            return;
        }
        await loadSettings();
    });

    let activeCategory = $derived(
        categories[activeTab as keyof typeof categories],
    );

    async function loadSettings() {
        try {
            await appLogo.refresh(getToken() || undefined);

            // 1. Fetch App Settings
            const data = await api.settings.getAll();

            // 2. Fetch Tenant Info
            tenantInfo = await api.tenant.getSelf();
            tenantChanges = {}; // Reset changes

            // 3. Check Custom Domain Access
            const access = await api.plans.checkAccess(
                tenantInfo.id,
                "custom_domain",
            );
            customDomainAccess = access.has_access;

            // Map settings
            settings = data.reduce(
                (acc, curr) => {
                    acc[curr.key] = curr;
                    return acc;
                },
                {} as Record<string, Setting>,
            );

            localSettings = {};
            // Populate defaults
            Object.values(categories).forEach((cat) => {
                cat.keys.forEach((key) => {
                    let val = settings[key]?.value ?? "";
                    if (key === "storage_driver" && !val) val = "system";
                    localSettings[key] = val;
                });
            });

            // Init Tenant local values
            localSettings["tenant_name"] = tenantInfo.name;
            localSettings["custom_domain"] = tenantInfo.custom_domain || "";
            localSettings["enforce_2fa"] = String(tenantInfo.enforce_2fa);

            // Init Bank Accounts
            loadBankAccounts();

            // Use current logo from store
            let logoStoreValue;
            appLogo.subscribe((v) => (logoStoreValue = v))();
            logoBase64 = logoStoreValue || null;

            hasChanges = false;
        } catch (error) {
            console.error(error);
            toast.error("Failed to load settings");
        } finally {
            loading = false;
        }
    }

    function handleChange(key: string, value: any) {
        localSettings[key] = String(value);

        // Check if tenant setting
        if (key === "tenant_name" || key === "custom_domain" || key === "enforce_2fa") {
            const originalName = tenantInfo?.name || "";
            const originalDomain = tenantInfo?.custom_domain || "";
            const originalEnforce = tenantInfo?.enforce_2fa ?? false;

            if (key === "tenant_name" && value !== originalName)
                tenantChanges.name = value;
            if (key === "custom_domain" && value !== originalDomain)
                tenantChanges.customDomain = value;
            if (key === "enforce_2fa" && Boolean(value) !== originalEnforce)
                (tenantChanges as any).enforce2fa = Boolean(value);

            // Revert if matches original
            if (key === "tenant_name" && value === originalName)
                delete tenantChanges.name;
            if (key === "custom_domain" && value === originalDomain)
                delete tenantChanges.customDomain;
            if (key === "enforce_2fa" && Boolean(value) === originalEnforce)
                delete (tenantChanges as any).enforce2fa;

            hasChanges = Object.keys(tenantChanges).length > 0;
        } else {
            const original = settings[key]?.value || "";
            hasChanges = String(value) !== original;
            // Simple check for now, can be improved to check all fields
        }

        localSettings = { ...localSettings };
    }

    async function handleFileUpload(e: Event) {
        const input = e.target as HTMLInputElement;
        if (!input.files || input.files.length === 0) return;
        const file = input.files[0];

        try {
            const reader = new FileReader();
            reader.onload = async (e) => {
                const base64 = e.target?.result as string;
                const base64Data = base64.split(",")[1];
                const path = await api.settings.uploadLogo(base64Data);

                localSettings["app_logo_path"] = path;
                appLogo.set(base64);
                logoBase64 = base64;
                hasChanges = true;
                toast.success("Logo uploaded");
            };
            reader.readAsDataURL(file);
        } catch (error) {
            toast.error("Failed to upload logo");
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
            if (activeTab !== "branding") {
                const keysToSave =
                    categories[activeTab as keyof typeof categories].keys;
                await Promise.all(
                    keysToSave.map((key) => {
                        if (key === "app_logo_path") return Promise.resolve();
                        const val = localSettings[key];
                        if (val !== undefined && val !== settings[key]?.value) {
                            // If locale changed, update immediately
                            if (key === "default_locale") {
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
            if (
                localSettings["default_locale"] !==
                settings["default_locale"]?.value
            ) {
                await waitLocale();
            }

            await loadSettings();
            toast.success("Settings saved");
        } catch (error: any) {
            toast.error(error.message || "Failed to save settings");
        } finally {
            saving = false;
        }
    }

    function discardChanges() {
        loadSettings();
    }

    // Input Helpers
    const localeOptions = [
        { value: "en", label: "English (US)" },
        { value: "id", label: "Bahasa Indonesia (ID)" },
    ];
    const currencyOptions = [
        { value: "Rp", label: "IDR (Rp)" },
        { value: "$", label: "USD ($)" },
    ];
    const storageOptions = [
        { value: "system", label: "System Default (Managed)" },
        { value: "s3", label: "AWS S3" },
        { value: "r2", label: "Cloudflare R2" },
    ];
    const emailProviderOptions = [
        { value: "smtp", label: "SMTP" },
        { value: "resend", label: "Resend API" },
    ];
    const smtpEncryptionOptions = [
        { value: "starttls", label: "STARTTLS" },
        { value: "tls", label: "TLS/SSL" },
        { value: "none", label: "None" },
    ];

    function getLabel(key: string) {
        return key.replace(/_/g, " ").replace(/\b\w/g, (l) => l.toUpperCase());
    }

    // Test Email State
    let testEmailAddress = $state("");
    let sendingTestEmail = $state(false);

    // Bank Account Management State
    let bankAccounts = $state<any[]>([]);
    let newBank = $state({
        bank_name: "",
        account_number: "",
        account_holder: "",
    });
    let showAddBank = $state(false);

    // Sync bankAccounts state with localSettings JSON string
    function loadBankAccounts() {
        try {
            const json = localSettings["payment_manual_accounts"];
            bankAccounts = json ? JSON.parse(json) : [];
        } catch (e) {
            bankAccounts = [];
        }
    }

    function addBankAccount() {
        if (
            !newBank.bank_name ||
            !newBank.account_number ||
            !newBank.account_holder
        )
            return;

        bankAccounts = [
            ...bankAccounts,
            { ...newBank, id: crypto.randomUUID() },
        ];
        newBank = { bank_name: "", account_number: "", account_holder: "" };
        showAddBank = false;

        // Update settings string
        handleChange("payment_manual_accounts", JSON.stringify(bankAccounts));
    }

    function removeBankAccount(id: string) {
        bankAccounts = bankAccounts.filter((b) => b.id !== id);
        handleChange("payment_manual_accounts", JSON.stringify(bankAccounts));
    }

    async function sendTestEmail() {
        if (!testEmailAddress) return;
        sendingTestEmail = true;
        try {
            const result = await api.settings.sendTestEmail(testEmailAddress);
            toast.success(result);
        } catch (error: any) {
            toast.error(error.message || "Failed to send test email");
        } finally {
            sendingTestEmail = false;
        }
    }

    // Plan Features Helper
    function getPlanFeatures(slug: string) {
        switch (slug) {
            case "free":
                return [
                    "Community Support",
                    "Basic Analytics",
                    "Subdomain Only",
                ];
            case "pro":
                return [
                    "Priority Support",
                    "Advanced Analytics",
                    "Custom Domain",
                    "Remove Branding",
                ];
            case "enterprise":
                return [
                    "24/7 Dedicated Support",
                    "Audit Logs",
                    "Custom Domain",
                    "SSO & Security",
                    "API Access",
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
                        <span class="icon"
                            ><Icon name={cat.icon} size={18} /></span
                        >
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
                            {categories[activeTab as keyof typeof categories]
                                .label}
                        </h2>
                        <p class="card-subtitle">
                            Manage your {activeTab} settings
                        </p>
                    </div>

                    <div class="settings-body">
                        {#if activeTab === "branding"}
                            <!-- Tenant Branding -->
                            <div class="setting-group">
                                <label for="tenant-name"
                                    >Organization Name</label
                                >
                                <Input
                                    id="tenant-name"
                                    value={localSettings["tenant_name"]}
                                    oninput={(e: any) =>
                                        handleChange(
                                            "tenant_name",
                                            e.target.value,
                                        )}
                                />
                            </div>

                            <div class="setting-group">
                                <label for="custom-domain">Custom Domain</label>
                                {#if customDomainAccess}
                                    <Input
                                        id="custom-domain"
                                        value={localSettings["custom_domain"]}
                                        oninput={(e: any) =>
                                            handleChange(
                                                "custom_domain",
                                                e.target.value,
                                            )}
                                        placeholder="e.g. app.yourcompany.com"
                                    />
                                    <p class="help-text">
                                        Point your domain's CNAME record to <code
                                            >cname.tridigitals.com</code
                                        > (or configured alias).
                                    </p>
                                {:else}
                                    <div class="upgrade-banner">
                                        <div class="icon-box">
                                            <Icon name="lock" size={20} />
                                        </div>
                                        <div class="text">
                                            <h4>
                                                Custom Domain is a Pro Feature
                                            </h4>
                                            <p>
                                                Upgrade your plan to use your
                                                own domain name.
                                            </p>
                                        </div>
                                        <button
                                            class="btn btn-primary btn-sm"
                                            onclick={() =>
                                                goto(
                                                    `/${tenantInfo?.slug}/admin/subscription`,
                                                )}
                                        >
                                            Upgrade Plan
                                        </button>
                                    </div>
                                    <Input
                                        value={localSettings["custom_domain"]}
                                        disabled={true}
                                        placeholder="Locked"
                                    />
                                {/if}
                            </div>
                        {:else if activeTab === "security"}
                            <!-- Security Settings -->
                            <div class="setting-item mt-6">
                                <div class="setting-info">
                                    <h3>Enforce Two-Factor Authentication</h3>
                                    <p>
                                        Require all members of this organization
                                        to enable 2FA before accessing the
                                        dashboard.
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        checked={localSettings["enforce_2fa"] === "true"}
                                        onchange={(e) =>
                                            handleChange(
                                                "enforce_2fa",
                                                e.currentTarget.checked,
                                            )}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                        {:else if activeTab === "storage"}
                            <!-- Redesigned Storage Settings -->
                            <div class="storage-settings">
                                <span class="section-label"
                                    >Select Storage Provider</span
                                >
                                <div class="provider-grid">
                                    {#each storageOptions as option}
                                        <button
                                            class="provider-card"
                                            class:selected={localSettings[
                                                "storage_driver"
                                            ] === option.value}
                                            onclick={() =>
                                                handleChange(
                                                    "storage_driver",
                                                    option.value,
                                                )}
                                        >
                                            <div class="p-icon">
                                                {#if option.value === "s3"}
                                                    <Icon
                                                        name="cloud"
                                                        size={24}
                                                    />
                                                {:else if option.value === "r2"}
                                                    <Icon
                                                        name="globe"
                                                        size={24}
                                                    />
                                                {:else}
                                                    <Icon
                                                        name="server"
                                                        size={24}
                                                    />
                                                {/if}
                                            </div>
                                            <div class="p-info">
                                                <span class="p-name"
                                                    >{option.label}</span
                                                >
                                                <span class="p-desc">
                                                    {#if option.value === "s3"}
                                                        Scalable object storage
                                                        by AWS.
                                                    {:else if option.value === "r2"}
                                                        Zero egress fee storage
                                                        by Cloudflare.
                                                    {:else}
                                                        Local disk storage
                                                        (Default).
                                                    {/if}
                                                </span>
                                            </div>
                                            <div class="p-check">
                                                <Icon
                                                    name={localSettings[
                                                        "storage_driver"
                                                    ] === option.value
                                                        ? "check-circle"
                                                        : "circle"}
                                                    size={20}
                                                />
                                            </div>
                                        </button>
                                    {/each}
                                </div>

                                {#if localSettings["storage_driver"] === "s3" || localSettings["storage_driver"] === "r2"}
                                    <div class="config-panel fade-in">
                                        <h3>Configuration</h3>
                                        <div class="config-grid">
                                            {#each categories["storage"].keys as key}
                                                {#if key !== "storage_driver"}
                                                    <div class="setting-item">
                                                        <label for={key}
                                                            >{getLabel(
                                                                key,
                                                            )}</label
                                                        >
                                                        <div
                                                            class="setting-control"
                                                        >
                                                            <Input
                                                                type={key.includes(
                                                                    "secret",
                                                                ) ||
                                                                key.includes(
                                                                    "key",
                                                                )
                                                                    ? "password"
                                                                    : "text"}
                                                                value={localSettings[
                                                                    key
                                                                ]}
                                                                oninput={(
                                                                    e: any,
                                                                ) =>
                                                                    handleChange(
                                                                        key,
                                                                        e.target
                                                                            .value,
                                                                    )}
                                                                placeholder={key.includes(
                                                                    "region",
                                                                )
                                                                    ? "e.g. us-east-1"
                                                                    : ""}
                                                            />
                                                        </div>
                                                    </div>
                                                {/if}
                                            {/each}
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        {:else if activeTab === "email"}
                            <!-- Redesigned Email Settings -->
                            <div class="email-settings">
                                <span class="section-label"
                                    >Email Delivery Provider</span
                                >
                                <div class="provider-grid">
                                    {#each emailProviderOptions as option}
                                        <button
                                            class="provider-card"
                                            class:selected={localSettings[
                                                "email_provider"
                                            ] === option.value}
                                            onclick={() =>
                                                handleChange(
                                                    "email_provider",
                                                    option.value,
                                                )}
                                        >
                                            <div class="p-icon">
                                                {#if option.value === "smtp"}
                                                    <Icon
                                                        name="mail"
                                                        size={24}
                                                    />
                                                {:else}
                                                    <Icon
                                                        name="zap"
                                                        size={24}
                                                    />
                                                {/if}
                                            </div>
                                            <div class="p-info">
                                                <span class="p-name"
                                                    >{option.label}</span
                                                >
                                                <span class="p-desc">
                                                    {#if option.value === "smtp"}
                                                        Direct SMTP server
                                                        connection.
                                                    {:else}
                                                        High-performance API
                                                        delivery.
                                                    {/if}
                                                </span>
                                            </div>
                                            <div class="p-check">
                                                <Icon
                                                    name={localSettings[
                                                        "email_provider"
                                                    ] === option.value
                                                        ? "check-circle"
                                                        : "circle"}
                                                    size={20}
                                                />
                                            </div>
                                        </button>
                                    {/each}
                                </div>

                                <div class="config-panel fade-in">
                                    <h3>Sender Information</h3>
                                    <div class="config-grid mb-6">
                                        <div class="setting-item">
                                            <label for="email-from-name"
                                                >From Name</label
                                            >
                                            <Input
                                                id="email-from-name"
                                                value={localSettings[
                                                    "email_from_name"
                                                ]}
                                                oninput={(e: any) =>
                                                    handleChange(
                                                        "email_from_name",
                                                        e.target.value,
                                                    )}
                                                placeholder="e.g. Acme Support"
                                            />
                                        </div>
                                        <div class="setting-item">
                                            <label for="email-from-address"
                                                >From Address</label
                                            >
                                            <Input
                                                id="email-from-address"
                                                value={localSettings[
                                                    "email_from_address"
                                                ]}
                                                oninput={(e: any) =>
                                                    handleChange(
                                                        "email_from_address",
                                                        e.target.value,
                                                    )}
                                                placeholder="noreply@yourdomain.com"
                                            />
                                        </div>
                                    </div>

                                    <div class="divider-line"></div>

                                    <h3 class="mt-6">Connection Details</h3>
                                    <div class="config-grid">
                                        {#if localSettings["email_provider"] === "smtp"}
                                            <div class="setting-item">
                                                <label for="smtp-host"
                                                    >SMTP Host</label
                                                >
                                                <Input
                                                    id="smtp-host"
                                                    value={localSettings[
                                                        "email_smtp_host"
                                                    ]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            "email_smtp_host",
                                                            e.target.value,
                                                        )}
                                                    placeholder="smtp.mailtrap.io"
                                                />
                                            </div>
                                            <div class="setting-item">
                                                <label for="smtp-port"
                                                    >SMTP Port</label
                                                >
                                                <Input
                                                    id="smtp-port"
                                                    type="number"
                                                    value={localSettings[
                                                        "email_smtp_port"
                                                    ]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            "email_smtp_port",
                                                            e.target.value,
                                                        )}
                                                    placeholder="587"
                                                />
                                            </div>
                                            <div class="setting-item">
                                                <label for="smtp-encryption"
                                                    >Encryption</label
                                                >
                                                <Select
                                                    id="smtp-encryption"
                                                    options={smtpEncryptionOptions}
                                                    value={localSettings[
                                                        "email_smtp_encryption"
                                                    ]}
                                                    onchange={(e: any) =>
                                                        handleChange(
                                                            "email_smtp_encryption",
                                                            e.detail,
                                                        )}
                                                />
                                            </div>
                                            <div class="setting-item">
                                                <label for="smtp-username"
                                                    >Username</label
                                                >
                                                <Input
                                                    id="smtp-username"
                                                    value={localSettings[
                                                        "email_smtp_username"
                                                    ]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            "email_smtp_username",
                                                            e.target.value,
                                                        )}
                                                />
                                            </div>
                                            <div
                                                class="setting-item full-width"
                                            >
                                                <label for="smtp-password"
                                                    >Password</label
                                                >
                                                <Input
                                                    id="smtp-password"
                                                    type="password"
                                                    value={localSettings[
                                                        "email_smtp_password"
                                                    ]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            "email_smtp_password",
                                                            e.target.value,
                                                        )}
                                                    placeholder="••••••••••••"
                                                    showPasswordToggle={true}
                                                />
                                            </div>
                                        {:else}
                                            <div
                                                class="setting-item full-width"
                                            >
                                                <label for="api-key"
                                                    >API Key</label
                                                >
                                                <Input
                                                    id="api-key"
                                                    type="password"
                                                    value={localSettings[
                                                        "email_api_key"
                                                    ]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            "email_api_key",
                                                            e.target.value,
                                                        )}
                                                    placeholder="re_123456789..."
                                                    showPasswordToggle={true}
                                                />
                                            </div>
                                        {/if}
                                    </div>
                                </div>

                                <div class="test-email-card mt-6">
                                    <div class="test-header">
                                        <Icon name="send" size={18} />
                                        <h4>Test Configuration</h4>
                                    </div>
                                    <p>
                                        Send a test email to verify your
                                        settings are working correctly.
                                    </p>
                                    <div class="test-form">
                                        <Input
                                            type="email"
                                            value={testEmailAddress}
                                            oninput={(e: any) =>
                                                (testEmailAddress =
                                                    e.target.value)}
                                            placeholder="Enter recipient email"
                                        />
                                        <button
                                            class="btn btn-secondary"
                                            onclick={sendTestEmail}
                                            disabled={sendingTestEmail ||
                                                !testEmailAddress}
                                        >
                                            {sendingTestEmail
                                                ? "Sending..."
                                                : "Send Test"}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        {:else if activeTab === "payment"}
                            <!-- Payment Settings -->
                            <div class="payment-settings">
                                <span class="section-label"
                                    >Payment Methods</span
                                >

                                <!-- Midtrans Card -->
                                <div class="method-card">
                                    <div class="method-header">
                                        <div class="m-icon midtrans">M</div>
                                        <div class="m-info">
                                            <h4>Midtrans Payment Gateway</h4>
                                            <p>
                                                Accept payments via Credit Card,
                                                GoPay, ShopeePay, VA, etc.
                                            </p>
                                        </div>
                                        <label class="toggle">
                                            <input
                                                type="checkbox"
                                                checked={localSettings[
                                                    "payment_midtrans_enabled"
                                                ] === "true"}
                                                onchange={(e) =>
                                                    handleChange(
                                                        "payment_midtrans_enabled",
                                                        e.currentTarget.checked,
                                                    )}
                                            />
                                            <span class="slider"></span>
                                        </label>
                                    </div>

                                    {#if localSettings["payment_midtrans_enabled"] === "true"}
                                        <div class="method-config fade-in">
                                            <div class="config-grid">
                                                <div class="setting-item">
                                                    <label
                                                        for="midtrans-merchant-id"
                                                        >Merchant ID</label
                                                    >
                                                    <Input
                                                        id="midtrans-merchant-id"
                                                        value={localSettings[
                                                            "payment_midtrans_merchant_id"
                                                        ]}
                                                        oninput={(e: any) =>
                                                            handleChange(
                                                                "payment_midtrans_merchant_id",
                                                                e.target.value,
                                                            )}
                                                        placeholder="G123456789"
                                                    />
                                                </div>
                                                <div class="setting-item">
                                                    <label
                                                        for="midtrans-client-key"
                                                        >Client Key</label
                                                    >
                                                    <Input
                                                        id="midtrans-client-key"
                                                        value={localSettings[
                                                            "payment_midtrans_client_key"
                                                        ]}
                                                        oninput={(e: any) =>
                                                            handleChange(
                                                                "payment_midtrans_client_key",
                                                                e.target.value,
                                                            )}
                                                        placeholder="SB-Mid-client-..."
                                                    />
                                                </div>
                                                <div
                                                    class="setting-item full-width"
                                                >
                                                    <label
                                                        for="midtrans-server-key"
                                                        >Server Key</label
                                                    >
                                                    <Input
                                                        id="midtrans-server-key"
                                                        type="password"
                                                        value={localSettings[
                                                            "payment_midtrans_server_key"
                                                        ]}
                                                        oninput={(e: any) =>
                                                            handleChange(
                                                                "payment_midtrans_server_key",
                                                                e.target.value,
                                                            )}
                                                        placeholder="SB-Mid-server-..."
                                                        showPasswordToggle={true}
                                                    />
                                                </div>
                                                <div
                                                    class="setting-item full-width checkbox-row"
                                                >
                                                    <label
                                                        class="checkbox-label"
                                                    >
                                                        <input
                                                            type="checkbox"
                                                            checked={localSettings[
                                                                "payment_midtrans_is_production"
                                                            ] === "true"}
                                                            onchange={(
                                                                e: any,
                                                            ) =>
                                                                handleChange(
                                                                    "payment_midtrans_is_production",
                                                                    e
                                                                        .currentTarget
                                                                        .checked,
                                                                )}
                                                        />
                                                        <span
                                                            >Enable Production
                                                            Mode (Live)</span
                                                        >
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
                                            <h4>Bank Transfer (Manual)</h4>
                                            <p>
                                                Accept payments via direct bank
                                                transfer verification.
                                            </p>
                                        </div>
                                        <label class="toggle">
                                            <input
                                                type="checkbox"
                                                checked={localSettings[
                                                    "payment_manual_enabled"
                                                ] === "true"}
                                                onchange={(e) =>
                                                    handleChange(
                                                        "payment_manual_enabled",
                                                        e.currentTarget.checked,
                                                    )}
                                            />
                                            <span class="slider"></span>
                                        </label>
                                    </div>

                                    {#if localSettings["payment_manual_enabled"] === "true"}
                                        <div class="method-config fade-in">
                                            <div
                                                class="setting-item full-width"
                                            >
                                                <label
                                                    for="payment-manual-instructions"
                                                    >Payment Instructions</label
                                                >
                                                <textarea
                                                    id="payment-manual-instructions"
                                                    class="form-textarea"
                                                    rows="4"
                                                    value={localSettings[
                                                        "payment_manual_instructions"
                                                    ]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            "payment_manual_instructions",
                                                            e.target.value,
                                                        )}
                                                    placeholder="Please transfer to BCA 1234567890 a/n PT Company..."
                                                ></textarea>
                                                <p class="help-text">
                                                    These instructions will be
                                                    shown to the user during
                                                    checkout.
                                                </p>
                                            </div>
                                            <div
                                                class="bank-accounts-manager mt-6"
                                            >
                                                <div class="bm-header">
                                                    <span class="label-text"
                                                        >Bank Accounts</span
                                                    >
                                                    <button
                                                        class="btn btn-primary btn-xs"
                                                        onclick={() =>
                                                            (showAddBank =
                                                                !showAddBank)}
                                                    >
                                                        <Icon
                                                            name={showAddBank
                                                                ? "minus"
                                                                : "plus"}
                                                            size={14}
                                                        />
                                                        {showAddBank
                                                            ? "Cancel"
                                                            : "Add Bank"}
                                                    </button>
                                                </div>

                                                {#if showAddBank}
                                                    <div
                                                        class="add-bank-form fade-in"
                                                    >
                                                        <div class="form-row">
                                                            <Input
                                                                aria-label="Bank Name"
                                                                value={newBank.bank_name}
                                                                oninput={(
                                                                    e: any,
                                                                ) =>
                                                                    (newBank.bank_name =
                                                                        e.target.value)}
                                                                placeholder="Bank Name (e.g. BCA)"
                                                            />
                                                            <Input
                                                                aria-label="Account Number"
                                                                value={newBank.account_number}
                                                                oninput={(
                                                                    e: any,
                                                                ) =>
                                                                    (newBank.account_number =
                                                                        e.target.value)}
                                                                placeholder="Account Number"
                                                            />
                                                        </div>
                                                        <div class="form-row">
                                                            <Input
                                                                aria-label="Account Holder Name"
                                                                value={newBank.account_holder}
                                                                oninput={(
                                                                    e: any,
                                                                ) =>
                                                                    (newBank.account_holder =
                                                                        e.target.value)}
                                                                placeholder="Account Holder Name"
                                                            />
                                                            <button
                                                                class="btn btn-secondary"
                                                                onclick={addBankAccount}
                                                                >Add</button
                                                            >
                                                        </div>
                                                    </div>
                                                {/if}

                                                <div class="bank-list-grid">
                                                    {#if bankAccounts.length === 0}
                                                        <div
                                                            class="empty-state"
                                                        >
                                                            <div
                                                                class="icon-placeholder"
                                                            >
                                                                <Icon
                                                                    name="landmark"
                                                                    size={24}
                                                                />
                                                            </div>
                                                            <p>
                                                                No bank accounts
                                                                added yet.
                                                            </p>
                                                            <button
                                                                class="btn btn-primary btn-sm mt-2"
                                                                onclick={() =>
                                                                    (showAddBank = true)}
                                                                >Add One</button
                                                            >
                                                        </div>
                                                    {:else}
                                                        {#each bankAccounts as bank}
                                                            <div
                                                                class="bank-card-item"
                                                            >
                                                                <div
                                                                    class="bc-icon"
                                                                >
                                                                    <Icon
                                                                        name="landmark"
                                                                        size={20}
                                                                    />
                                                                </div>
                                                                <div
                                                                    class="bc-details"
                                                                >
                                                                    <span
                                                                        class="bc-name"
                                                                        >{bank.bank_name}</span
                                                                    >
                                                                    <span
                                                                        class="bc-number"
                                                                        >{bank.account_number}</span
                                                                    >
                                                                    <span
                                                                        class="bc-holder"
                                                                        >{bank.account_holder}</span
                                                                    >
                                                                </div>
                                                                <div
                                                                    class="bc-actions"
                                                                >
                                                                    <button
                                                                        class="btn-icon delete"
                                                                        onclick={() =>
                                                                            removeBankAccount(
                                                                                bank.id,
                                                                            )}
                                                                        title="Remove Account"
                                                                    >
                                                                        <Icon
                                                                            name="trash"
                                                                            size={16}
                                                                        />
                                                                    </button>
                                                                </div>
                                                            </div>
                                                        {/each}
                                                        <button
                                                            class="add-bank-card"
                                                            onclick={() =>
                                                                (showAddBank = true)}
                                                        >
                                                            <Icon
                                                                name="plus"
                                                                size={24}
                                                            />
                                                            <span
                                                                >Add Account</span
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
                                            <label for={key}
                                                >{getLabel(key)}</label
                                            >
                                        </div>
                                        <div class="setting-control">
                                            {#if key === "app_logo_path"}
                                                <div class="file-upload">
                                                    {#if logoBase64}
                                                        <img
                                                            src={logoBase64}
                                                            class="logo-preview"
                                                            alt="Logo"
                                                        />
                                                    {/if}
                                                    <input
                                                        type="file"
                                                        accept="image/*"
                                                        onchange={handleFileUpload}
                                                    />
                                                </div>
                                            {:else if key.includes("password") || key.includes("secret") || key.includes("key")}
                                                <Input
                                                    id={key}
                                                    type="password"
                                                    value={localSettings[key]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            key,
                                                            e.target.value,
                                                        )}
                                                />
                                            {:else if key === "default_locale"}
                                                <Select
                                                    id={key}
                                                    options={localeOptions}
                                                    value={localSettings[key]}
                                                    onchange={(e: any) =>
                                                        handleChange(
                                                            key,
                                                            e.detail,
                                                        )}
                                                />
                                            {:else if key === "currency_symbol"}
                                                <Select
                                                    id={key}
                                                    options={currencyOptions}
                                                    value={localSettings[key]}
                                                    onchange={(e: any) =>
                                                        handleChange(
                                                            key,
                                                            e.detail,
                                                        )}
                                                />
                                            {:else if key === "storage_driver"}
                                                <Select
                                                    id={key}
                                                    options={storageOptions}
                                                    value={localSettings[key]}
                                                    onchange={(e: any) =>
                                                        handleChange(
                                                            key,
                                                            e.detail,
                                                        )}
                                                />
                                            {:else if key === "email_provider"}
                                                <Select
                                                    id={key}
                                                    options={emailProviderOptions}
                                                    value={localSettings[key]}
                                                    onchange={(e: any) =>
                                                        handleChange(
                                                            key,
                                                            e.detail,
                                                        )}
                                                />
                                            {:else if key === "email_smtp_encryption"}
                                                <Select
                                                    id={key}
                                                    options={smtpEncryptionOptions}
                                                    value={localSettings[key]}
                                                    onchange={(e: any) =>
                                                        handleChange(
                                                            key,
                                                            e.detail,
                                                        )}
                                                />
                                            {:else}
                                                <Input
                                                    id={key}
                                                    value={localSettings[key]}
                                                    oninput={(e: any) =>
                                                        handleChange(
                                                            key,
                                                            e.target.value,
                                                        )}
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
                            onclick={discardChanges}>Reset</button
                        >
                        <button
                            class="btn btn-primary"
                            disabled={!hasChanges || saving}
                            onclick={saveChanges}
                        >
                            {saving ? "Saving..." : "Save Changes"}
                        </button>
                    </div>
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
        border-radius: 12px;
        padding: 1rem;
        position: sticky;
        top: 2rem;
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
        font-weight: 500;
        cursor: pointer;
        border-radius: 8px;
        text-align: left;
    }
    .nav-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }
    .nav-item.active {
        background: rgba(99, 102, 241, 0.1);
        color: var(--color-primary);
        font-weight: 600;
    }

    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        overflow: hidden;
    }
    .card-header {
        padding: 1.5rem;
        border-bottom: 1px solid var(--border-color);
    }
    .card-title {
        font-size: 1.25rem;
        font-weight: 600;
        margin: 0;
    }
    .card-subtitle {
        color: var(--text-secondary);
        font-size: 0.9rem;
        margin: 0.25rem 0 0;
    }

    .settings-body {
        padding: 2rem;
    }
    .settings-list {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 2rem;
    }
    .setting-item {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .setting-info label {
        font-weight: 500;
        color: var(--text-primary);
    }

    .setting-group {
        margin-bottom: 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .setting-group label {
        font-weight: 500;
    }
    .help-text {
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin-top: 0.25rem;
    }
    code {
        background: var(--bg-tertiary);
        padding: 0.1rem 0.3rem;
        border-radius: 4px;
        font-family: monospace;
    }

    .upgrade-banner {
        background: linear-gradient(
            135deg,
            rgba(99, 102, 241, 0.1),
            rgba(236, 72, 153, 0.1)
        );
        border: 1px solid var(--color-primary-subtle);
        border-radius: 8px;
        padding: 1rem;
        display: flex;
        align-items: center;
        gap: 1rem;
        margin-bottom: 1rem;
    }
    .icon-box {
        color: var(--color-primary);
    }
    .upgrade-banner .text {
        flex: 1;
    }
    .upgrade-banner h4 {
        margin: 0;
        font-size: 0.95rem;
        font-weight: 600;
    }
    .upgrade-banner p {
        margin: 0;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .card-footer {
        padding: 1.5rem;
        background: var(--bg-tertiary);
        border-top: 1px solid var(--border-color);
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
    }

    .btn {
        padding: 0.6rem 1.2rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        border: none;
    }
    .btn-primary {
        background: var(--color-primary);
        color: white;
    }
    .btn-primary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .btn-secondary {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
    }
    .btn-sm {
        padding: 0.4rem 0.8rem;
        font-size: 0.85rem;
    }

    .logo-preview {
        width: 48px;
        height: 48px;
        object-fit: contain;
        border-radius: 6px;
        border: 1px solid var(--border-color);
        margin-bottom: 0.5rem;
    }
    .loading-state {
        padding: 4rem;
        display: flex;
        justify-content: center;
    }
    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
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
        .settings-list {
            grid-template-columns: 1fr;
        }
    }

    /* Storage UI */
    .section-label {
        font-weight: 600;
        color: var(--text-primary);
        margin-bottom: 1rem;
        display: block;
    }
    .provider-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1rem;
        margin-bottom: 2rem;
    }

    .provider-card {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1.25rem;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        text-align: left;
        cursor: pointer;
        transition: all 0.2s;
        position: relative;
    }
    .provider-card:hover {
        border-color: var(--color-primary);
        background: var(--bg-hover);
    }
    .provider-card.selected {
        border: 2px solid var(--color-primary);
        background: var(--color-primary-subtle);
    }

    .p-icon {
        width: 40px;
        height: 40px;
        border-radius: 8px;
        background: var(--bg-tertiary);
        color: var(--text-secondary);
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
    }
    .selected .p-icon {
        background: white;
        color: var(--color-primary);
    }

    .p-info {
        flex: 1;
        display: flex;
        flex-direction: column;
    }
    .p-name {
        font-weight: 600;
        color: var(--text-primary);
    }
    .p-desc {
        font-size: 0.8rem;
        color: var(--text-secondary);
    }

    .p-check {
        color: var(--border-color);
    }
    .selected .p-check {
        color: var(--color-primary);
    }

    .config-panel {
        background: var(--bg-tertiary);
        padding: 1.5rem;
        border-radius: 12px;
        border: 1px solid var(--border-color);
    }
    .config-panel h3 {
        margin: 0 0 1.5rem 0;
        font-size: 1rem;
    }
    .config-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }

    .divider-line {
        height: 1px;
        background: var(--border-color);
        margin: 2rem 0;
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
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.5rem;
    }
    .test-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--color-primary);
        margin-bottom: 0.5rem;
    }
    .test-header h4 {
        margin: 0;
        font-size: 1rem;
        font-weight: 600;
    }
    .test-email-card p {
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin-bottom: 1.25rem;
    }
    .test-form {
        display: flex;
        gap: 1rem;
    }
    .test-form :global(.input-wrapper) {
        flex: 1;
    }

    @media (max-width: 640px) {
        .config-grid {
            grid-template-columns: 1fr;
        }
        .test-form {
            flex-direction: column;
        }
    }

    /* Payment UI */
    .method-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        overflow: hidden;
    }
    .method-header {
        padding: 1.25rem;
        display: flex;
        align-items: center;
        gap: 1rem;
        background: var(--bg-surface);
    }
    .m-icon {
        width: 40px;
        height: 40px;
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 700;
        font-size: 1.2rem;
        flex-shrink: 0;
    }
    .m-icon.midtrans {
        background: #002c5f;
        color: white;
    }
    .m-icon.manual {
        background: var(--bg-tertiary);
        color: var(--text-secondary);
    }

    .m-info {
        flex: 1;
    }
    .m-info h4 {
        margin: 0;
        font-size: 1rem;
        color: var(--text-primary);
    }
    .m-info p {
        margin: 0;
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .method-config {
        padding: 1.5rem;
        background: var(--bg-tertiary);
        border-top: 1px solid var(--border-color);
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
    }

    .form-textarea {
        width: 100%;
        padding: 0.75rem;
        border-radius: 8px;
        border: 1px solid var(--border-color);
        background: var(--bg-surface);
        color: var(--text-primary);
        font-family: inherit;
        font-size: 0.9rem;
        resize: vertical;
        transition: border-color 0.2s;
    }
    .form-textarea:focus {
        outline: none;
        border-color: var(--color-primary);
    }

    /* Bank Manager UI */
    .bank-accounts-manager {
        margin-top: 1.5rem;
        border-top: 1px dashed var(--border-color);
        padding-top: 1.5rem;
    }
    .bm-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }

    .bank-list-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
        gap: 1rem;
    }

    .bank-card-item {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.25rem;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        position: relative;
        transition: all 0.2s;
    }
    .bank-card-item:hover {
        border-color: var(--color-primary);
        box-shadow: var(--shadow-sm);
    }

    .bc-icon {
        width: 36px;
        height: 36px;
        background: var(--bg-tertiary);
        border-radius: 8px;
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--text-secondary);
    }
    .bc-details {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    .bc-name {
        font-weight: 700;
        color: var(--text-primary);
        font-size: 0.95rem;
    }
    .bc-number {
        font-family: monospace;
        font-size: 1rem;
        letter-spacing: 0.05em;
        color: var(--text-primary);
    }
    .bc-holder {
        font-size: 0.8rem;
        color: var(--text-secondary);
        text-transform: uppercase;
    }

    .bc-actions {
        position: absolute;
        top: 1rem;
        right: 1rem;
    }

    .add-bank-card {
        border: 2px dashed var(--border-color);
        background: transparent;
        border-radius: 12px;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        min-height: 140px;
        color: var(--text-secondary);
        cursor: pointer;
        transition: all 0.2s;
    }
    .add-bank-card:hover {
        border-color: var(--color-primary);
        color: var(--color-primary);
        background: var(--color-primary-subtle);
    }
    .add-bank-card span {
        font-weight: 600;
        font-size: 0.9rem;
    }

    .empty-state {
        grid-column: 1 / -1;
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 2rem;
        background: var(--bg-surface);
        border-radius: 12px;
        border: 1px solid var(--border-color);
    }
    .icon-placeholder {
        width: 48px;
        height: 48px;
        background: var(--bg-tertiary);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-bottom: 1rem;
        color: var(--text-secondary);
    }
</style>

<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { user, isAdmin, getToken } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { goto } from "$app/navigation";
    import { locale, t } from "svelte-i18n";
    import Icon from "$lib/components/Icon.svelte";
    import type { Setting } from "$lib/api/client";

    let loading = true;
    let saving = false;
    let settings: Record<string, Setting> = {};
    let localSettings: Record<string, string> = {};
    let logoBase64: string | null = null;
    let activeTab = "general";
    let message = { type: "", text: "" };
    let hasChanges = false;
    let showMobileMenu = false;

    // Options
    const localeOptions = [
        { value: "en", label: "English (US)" },
        { value: "id", label: "Bahasa Indonesia (ID)" },
    ];

    const currencyOptions = [
        { value: "Rp", label: "IDR (Rp)" },
        { value: "$", label: "USD ($)" },
    ];

    const storageOptions = [
        { value: "local", label: "Local File System" },
        { value: "s3", label: "AWS S3 / Compatible" },
        { value: "r2", label: "Cloudflare R2" },
    ];

    const emailProviderOptions = [
        { value: "smtp", label: "SMTP (Direct)" },
        { value: "resend", label: "Resend API" },
        { value: "sendgrid", label: "SendGrid API" },
        { value: "webhook", label: "Custom Webhook" },
    ];

    const smtpEncryptionOptions = [
        { value: "starttls", label: "STARTTLS (Port 587)" },
        { value: "tls", label: "TLS/SSL (Port 465)" },
        { value: "none", label: "None (Not Secure)" },
    ];

    let testEmailAddress = "";
    let sendingTestEmail = false;

    // Categories configuration
    const categories = {
        general: {
            label: "General",
            icon: "app",
            keys: [
                "app_name",
                "app_description",
                "organization_name",
                "support_email",
                "default_locale",
                "currency_symbol",
                "maintenance_mode",
                "app_version",
                "app_logo_path",
            ],
        },
        storage: {
            label: "Storage",
            icon: "database",
            keys: [
                "storage_provider",
                "storage_local_path",
                "storage_s3_bucket",
                "storage_s3_region",
                "storage_s3_access_key",
                "storage_s3_secret_key",
                "storage_s3_endpoint",
                "storage_r2_bucket",
                "storage_r2_account_id",
                "storage_r2_access_key",
                "storage_r2_secret_key",
                "storage_r2_public_url",
            ],
        },
        auth: {
            label: "Authentication",
            icon: "lock",
            keys: [
                "auth_allow_registration",
                "auth_require_email_verification",
                "auth_jwt_expiry_hours",
                "auth_session_timeout_minutes",
            ],
        },
        security: {
            label: "Security & Policy",
            icon: "shield",
            keys: [
                "auth_password_min_length",
                "auth_password_require_uppercase",
                "auth_password_require_number",
                "auth_password_require_special",
                "auth_max_login_attempts",
                "auth_lockout_duration_minutes",
                "auth_logout_all_on_password_change",
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
    };

    onMount(async () => {
        if (!$isAdmin) {
            goto("/unauthorized");
            return;
        }
        await loadSettings();
    });

    async function loadSettings() {
        try {
            // Refresh logo from backend to sync between browser/desktop
            await appLogo.refresh(getToken() || undefined);

            // Fetch settings
            const data = await api.settings.getAll();

            // Use current logo from store (now refreshed)
            let logoStoreValue;
            appLogo.subscribe((v) => (logoStoreValue = v))();
            logoBase64 = logoStoreValue;

            settings = data.reduce(
                (acc, curr) => {
                    acc[curr.key] = curr;
                    return acc;
                },
                {} as Record<string, Setting>,
            );

            localSettings = {};
            Object.values(categories).forEach((cat) => {
                cat.keys.forEach((key) => {
                    let val = settings[key]?.value;
                    if (val === undefined || val === null) {
                        // Set defaults for specific keys
                        if (key === "storage_provider") val = "local";
                        else if (key === "auth_logout_all_on_password_change")
                            val = "true";
                        else val = "";
                    }
                    localSettings[key] = val;
                    if (!settings[key]) {
                        settings[key] = {
                            key,
                            value: val,
                            description: "",
                            id: "",
                            created_at: "",
                            updated_at: "",
                        };
                    }
                });
            });
            hasChanges = false;
        } catch (error) {
            console.error(error);
            showMessage("error", "Failed to load settings");
        } finally {
            loading = false;
        }
    }

    function handleChange(key: string, value: any) {
        localSettings[key] = String(value);
        const original = settings[key]?.value || "";
        hasChanges =
            String(value) !== original ||
            Object.keys(localSettings).some(
                (k) => localSettings[k] !== (settings[k]?.value || ""),
            );
        localSettings = { ...localSettings };
    }

    // Compress and resize image to optimize for logo usage
    async function compressImage(
        file: File,
        maxWidth: number = 256,
        maxHeight: number = 256,
        quality: number = 0.9,
    ): Promise<string> {
        return new Promise((resolve, reject) => {
            const img = new window.Image();
            const reader = new FileReader();

            reader.onload = (e) => {
                img.onload = () => {
                    // Calculate new dimensions while preserving aspect ratio
                    let width = img.width;
                    let height = img.height;

                    if (width > maxWidth) {
                        height = (height * maxWidth) / width;
                        width = maxWidth;
                    }
                    if (height > maxHeight) {
                        width = (width * maxHeight) / height;
                        height = maxHeight;
                    }

                    // Create canvas and draw resized image
                    const canvas = document.createElement("canvas");
                    canvas.width = width;
                    canvas.height = height;
                    const ctx = canvas.getContext("2d");

                    if (!ctx) {
                        reject(new Error("Failed to get canvas context"));
                        return;
                    }

                    // Use high quality rendering
                    ctx.imageSmoothingEnabled = true;
                    ctx.imageSmoothingQuality = "high";
                    ctx.drawImage(img, 0, 0, width, height);

                    // Export as PNG (or JPEG for photos)
                    const compressedBase64 = canvas.toDataURL(
                        "image/png",
                        quality,
                    );

                    console.log(
                        `[Compress] Original: ${file.size} bytes, Resized to: ${width}x${height}`,
                    );

                    resolve(compressedBase64);
                };

                img.onerror = () => reject(new Error("Failed to load image"));
                img.src = e.target?.result as string;
            };

            reader.onerror = () => reject(new Error("Failed to read file"));
            reader.readAsDataURL(file);
        });
    }

    async function handleFileUpload(e: Event) {
        const input = e.target as HTMLInputElement;
        if (!input.files || input.files.length === 0) return;

        const file = input.files[0];
        console.log(
            `[Upload] File selected: ${file.name}, Size: ${file.size} bytes`,
        );

        if (file.size > 5 * 1024 * 1024) {
            showMessage("error", "File size too large (Max 5MB)");
            return;
        }

        try {
            // Compress and resize image before upload
            const compressedBase64 = await compressImage(file, 256, 256);
            logoBase64 = compressedBase64;

            // Extract base64 data (remove data:image/png;base64, prefix)
            const base64Data = compressedBase64.split(",")[1];

            console.log(`[Upload] Sending compressed image to backend...`);

            const path = await api.settings.uploadLogo(base64Data);
            console.log(`[Upload] Success! Path: ${path}`);

            localSettings["app_logo_path"] = path;
            localSettings = { ...localSettings };

            hasChanges = true;

            appSettings.updateSetting("app_logo_path", path);
            appLogo.set(compressedBase64); // Update global logo store

            showMessage("success", "Logo uploaded and optimized");
        } catch (error) {
            console.error("[Upload] Error:", error);
            showMessage("error", "Failed to upload logo: " + String(error));
        }
    }

    async function saveChanges() {
        saving = true;
        try {
            const keysToSave = categories[activeTab].keys;

            await Promise.all(
                keysToSave.map((key) => {
                    if (key === "app_logo_path") return Promise.resolve();

                    const val = localSettings[key];
                    if (val !== undefined) {
                        appSettings.updateSetting(key, val);

                        // If locale changed, update immediately
                        if (key === "default_locale") {
                            locale.set(val);
                        }

                        return api.settings.upsert(key, val);
                    }
                }),
            );

            await loadSettings();
            showMessage("success", "Changes saved successfully");
        } catch (error) {
            console.error(error);
            showMessage("error", "Failed to save settings");
        } finally {
            saving = false;
        }
    }

    function discardChanges() {
        Object.keys(localSettings).forEach((k) => {
            localSettings[k] = settings[k]?.value || "";
        });

        // Reset logo preview to current actual logo
        let logoStoreValue;
        appLogo.subscribe((v) => (logoStoreValue = v))();
        logoBase64 = logoStoreValue;

        hasChanges = false;
    }

    function showMessage(type: "success" | "error", text: string) {
        message = { type, text };
        setTimeout(() => (message = { type: "", text: "" }), 3000);
    }

    async function sendTestEmail() {
        if (!testEmailAddress) {
            showMessage("error", "Please enter an email address");
            return;
        }
        sendingTestEmail = true;
        try {
            const result = await api.settings.sendTestEmail(testEmailAddress);
            showMessage("success", result);
        } catch (error) {
            console.error(error);
            showMessage("error", "Failed to send test email: " + String(error));
        } finally {
            sendingTestEmail = false;
        }
    }

    function getInputType(key: string) {
        if (key === "default_locale") return "select-locale";
        if (key === "currency_symbol") return "select-currency";
        if (key === "storage_provider") return "select-storage";
        if (key === "email_provider") return "select-email-provider";
        if (key === "email_smtp_encryption") return "select-smtp-encryption";
        if (key === "app_logo_path") return "file";
        if (key === "email_smtp_port") return "number";
        if (
            key.includes("secret") ||
            key.includes("api_key") ||
            (key.includes("password") &&
                !key.includes("min_length") &&
                !key.includes("require") &&
                !key.includes("logout_all"))
        )
            return "password";

        if (
            key.includes("hours") ||
            key.includes("minutes") ||
            key.includes("length") ||
            key.includes("attempts")
        )
            return "number";

        if (
            key.includes("allow") ||
            key.includes("require") ||
            key.includes("maintenance") ||
            key.includes("logout_all")
        )
            return "boolean";

        return "text";
    }

    function isFieldVisible(key: string) {
        // Storage provider visibility
        if (key === "storage_provider") return true;
        const storageProvider = localSettings["storage_provider"] || "local";
        if (key.startsWith("storage_local_") && storageProvider !== "local")
            return false;
        if (key.startsWith("storage_s3_") && storageProvider !== "s3")
            return false;
        if (key.startsWith("storage_r2_") && storageProvider !== "r2")
            return false;

        // Email provider visibility
        if (key === "email_provider") return true;
        if (key === "email_from_address" || key === "email_from_name")
            return true;

        const emailProvider = localSettings["email_provider"] || "smtp";

        // SMTP fields - only show for SMTP provider
        const smtpFields = [
            "email_smtp_host",
            "email_smtp_port",
            "email_smtp_username",
            "email_smtp_password",
            "email_smtp_encryption",
        ];
        if (smtpFields.includes(key) && emailProvider !== "smtp") return false;

        // API key - show for resend and sendgrid
        if (
            key === "email_api_key" &&
            !["resend", "sendgrid"].includes(emailProvider)
        )
            return false;

        // Webhook URL - only for webhook
        if (key === "email_webhook_url" && emailProvider !== "webhook")
            return false;

        return true;
    }

    function getLabel(key: string) {
        // Try to translate key
        return $t(`admin.settings.keys.${key}`);
    }
</script>

<div class="page-container fade-in">
    {#if message.text}
        <div class="alert alert-{message.type} slide-in">
            {message.text}
        </div>
    {/if}

    <div class="layout-grid">
        <!-- Desktop Sidebar -->
        <aside class="sidebar card desktop-sidebar">
            <nav>
                {#each Object.entries(categories) as [id, cat]}
                    <button
                        class="nav-item {activeTab === id ? 'active' : ''}"
                        on:click={() => {
                            activeTab = id;
                            discardChanges();
                        }}
                    >
                        <span class="icon">
                            <Icon name={cat.icon} size={18} />
                        </span>
                        {$t(`admin.settings.categories.${id}`)}
                    </button>
                {/each}
            </nav>
        </aside>

        <!-- Mobile FAB & Menu -->
        <button
            class="mobile-fab"
            on:click={() => (showMobileMenu = !showMobileMenu)}
            aria-label="Settings Menu"
        >
            <Icon name={showMobileMenu ? "x" : "settings"} size={24} />
        </button>

        {#if showMobileMenu}
            <div
                class="mobile-menu-overlay"
                on:click={() => (showMobileMenu = false)}
            >
                <div class="mobile-menu card slide-up" on:click|stopPropagation>
                    <div class="mobile-menu-header">
                        <h3>Settings</h3>
                        <button
                            class="close-btn"
                            on:click={() => (showMobileMenu = false)}
                        >
                            <Icon name="x" size={20} />
                        </button>
                    </div>
                    <nav>
                        {#each Object.entries(categories) as [id, cat]}
                            <button
                                class="nav-item {activeTab === id
                                    ? 'active'
                                    : ''}"
                                on:click={() => {
                                    activeTab = id;
                                    showMobileMenu = false;
                                    discardChanges();
                                }}
                            >
                                <span class="icon">
                                    <Icon name={cat.icon} size={18} />
                                </span>
                                {$t(`admin.settings.categories.${id}`)}
                            </button>
                        {/each}
                    </nav>
                </div>
            </div>
        {/if}

        <main class="content">
            {#if loading}
                <div class="loading-state">Loading settings...</div>
            {:else}
                <div class="card section fade-in">
                    <div class="card-header">
                        <div class="header-text">
                            <h2 class="card-title">
                                {$t(`admin.settings.categories.${activeTab}`)}
                            </h2>
                            <p class="card-subtitle">
                                {$t("admin.settings.subtitle")}
                            </p>
                        </div>
                    </div>

                    <div class="settings-list">
                        {#each categories[activeTab].keys as key}
                            {#if isFieldVisible(key)}
                                <div class="setting-item">
                                    <div class="setting-info">
                                        <label for={key}>{getLabel(key)}</label>
                                        {#if settings[key]?.description}
                                            <p class="description">
                                                {settings[key].description}
                                            </p>
                                        {/if}
                                    </div>

                                    <div class="setting-control">
                                        {#if getInputType(key) === "boolean"}
                                            <label class="toggle">
                                                <input
                                                    type="checkbox"
                                                    id={key}
                                                    checked={localSettings[
                                                        key
                                                    ] === "true"}
                                                    on:change={(e) =>
                                                        handleChange(
                                                            key,
                                                            e.currentTarget
                                                                .checked,
                                                        )}
                                                    disabled={saving}
                                                />
                                                <span class="slider"></span>
                                            </label>
                                        {:else if getInputType(key) === "select-locale"}
                                            <div class="select-wrapper">
                                                <select
                                                    id={key}
                                                    class="form-input"
                                                    value={localSettings[key]}
                                                    on:change={(e) =>
                                                        handleChange(
                                                            key,
                                                            e.currentTarget
                                                                .value,
                                                        )}
                                                    disabled={saving}
                                                >
                                                    {#each localeOptions as option}
                                                        <option
                                                            value={option.value}
                                                            >{option.label}</option
                                                        >
                                                    {/each}
                                                </select>
                                            </div>
                                        {:else if getInputType(key) === "select-currency"}
                                            <div class="select-wrapper">
                                                <select
                                                    id={key}
                                                    class="form-input"
                                                    value={localSettings[key]}
                                                    on:change={(e) =>
                                                        handleChange(
                                                            key,
                                                            e.currentTarget
                                                                .value,
                                                        )}
                                                    disabled={saving}
                                                >
                                                    {#each currencyOptions as option}
                                                        <option
                                                            value={option.value}
                                                            >{option.label}</option
                                                        >
                                                    {/each}
                                                </select>
                                            </div>
                                        {:else if getInputType(key) === "select-storage"}
                                            <div class="select-wrapper">
                                                <select
                                                    id={key}
                                                    class="form-input"
                                                    value={localSettings[key]}
                                                    on:change={(e) =>
                                                        handleChange(
                                                            key,
                                                            e.currentTarget
                                                                .value,
                                                        )}
                                                    disabled={saving}
                                                >
                                                    {#each storageOptions as option}
                                                        <option
                                                            value={option.value}
                                                            >{option.label}</option
                                                        >
                                                    {/each}
                                                </select>
                                            </div>
                                        {:else if getInputType(key) === "select-email-provider"}
                                            <div class="select-wrapper">
                                                <select
                                                    id={key}
                                                    class="form-input"
                                                    value={localSettings[key]}
                                                    on:change={(e) =>
                                                        handleChange(
                                                            key,
                                                            e.currentTarget
                                                                .value,
                                                        )}
                                                    disabled={saving}
                                                >
                                                    {#each emailProviderOptions as option}
                                                        <option
                                                            value={option.value}
                                                            >{option.label}</option
                                                        >
                                                    {/each}
                                                </select>
                                            </div>
                                        {:else if getInputType(key) === "select-smtp-encryption"}
                                            <div class="select-wrapper">
                                                <select
                                                    id={key}
                                                    class="form-input"
                                                    value={localSettings[key]}
                                                    on:change={(e) =>
                                                        handleChange(
                                                            key,
                                                            e.currentTarget
                                                                .value,
                                                        )}
                                                    disabled={saving}
                                                >
                                                    {#each smtpEncryptionOptions as option}
                                                        <option
                                                            value={option.value}
                                                            >{option.label}</option
                                                        >
                                                    {/each}
                                                </select>
                                            </div>
                                        {:else if getInputType(key) === "number"}
                                            <input
                                                type="number"
                                                id={key}
                                                class="form-input number-input"
                                                value={localSettings[key]}
                                                on:input={(e) =>
                                                    handleChange(
                                                        key,
                                                        e.currentTarget.value,
                                                    )}
                                                disabled={saving}
                                            />
                                        {:else if getInputType(key) === "password"}
                                            <input
                                                type="password"
                                                id={key}
                                                class="form-input"
                                                value={localSettings[key]}
                                                on:input={(e) =>
                                                    handleChange(
                                                        key,
                                                        e.currentTarget.value,
                                                    )}
                                                disabled={saving}
                                                placeholder="••••••••••••••"
                                            />
                                        {:else if getInputType(key) === "file"}
                                            <div class="file-upload-wrapper">
                                                {#if logoBase64}
                                                    <img
                                                        src={logoBase64}
                                                        alt="Logo Preview"
                                                        class="logo-preview"
                                                    />
                                                {/if}
                                                <input
                                                    type="file"
                                                    id={key}
                                                    accept="image/png, image/jpeg, image/svg+xml"
                                                    on:change={handleFileUpload}
                                                    disabled={saving}
                                                    class="file-input"
                                                />
                                            </div>
                                        {:else}
                                            <input
                                                type="text"
                                                id={key}
                                                class="form-input"
                                                value={localSettings[key]}
                                                on:input={(e) =>
                                                    handleChange(
                                                        key,
                                                        e.currentTarget.value,
                                                    )}
                                                disabled={saving ||
                                                    key === "app_version"}
                                                readonly={key === "app_version"}
                                            />
                                        {/if}
                                    </div>
                                </div>
                            {/if}
                        {/each}
                    </div>

                    {#if activeTab === "email"}
                        <div class="test-email-section">
                            <h3>{$t("admin.settings.test_email.title")}</h3>
                            <p class="section-description">
                                {$t("admin.settings.test_email.description")}
                            </p>
                            <div class="test-email-form">
                                <input
                                    type="email"
                                    class="form-input"
                                    placeholder={$t(
                                        "admin.settings.test_email.placeholder",
                                    )}
                                    bind:value={testEmailAddress}
                                    disabled={sendingTestEmail}
                                />
                                <button
                                    class="btn btn-primary"
                                    on:click={sendTestEmail}
                                    disabled={sendingTestEmail ||
                                        !testEmailAddress}
                                >
                                    {#if sendingTestEmail}{$t(
                                            "admin.settings.test_email.sending",
                                        )}{:else}{$t(
                                            "admin.settings.test_email.button",
                                        )}{/if}
                                </button>
                            </div>
                        </div>
                    {/if}

                    <div class="card-footer">
                        <button
                            class="btn btn-secondary"
                            disabled={!hasChanges || saving}
                            on:click={discardChanges}
                        >
                            {$t("admin.settings.reset_button")}
                        </button>
                        <button
                            class="btn btn-primary"
                            disabled={!hasChanges || saving}
                            on:click={saveChanges}
                        >
                            {#if saving}{$t("admin.settings.saving")}{:else}{$t(
                                    "admin.settings.save_button",
                                )}{/if}
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
        border-radius: var(--radius-lg);
        padding: 1rem;
        position: sticky;
        top: 2rem;
        z-index: 10;
    }

    /* Media query moved to bottom */

    /* Mobile FAB Styles */
    .mobile-fab {
        position: fixed;
        bottom: 2rem;
        right: 2rem;
        width: 3.5rem;
        height: 3.5rem;
        border-radius: 50%;
        background: var(--color-primary);
        color: white;
        border: none;
        box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
        display: none; /* Hidden by default on desktop */
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 100;
        transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
    }

    .mobile-fab:active {
        transform: scale(0.9);
    }

    /* Mobile Menu Overlay Styles */
    .mobile-menu-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        backdrop-filter: blur(4px);
        z-index: 90;
        display: flex;
        align-items: flex-end;
        justify-content: center;
        padding: 1rem;
    }

    .mobile-menu {
        width: 100%;
        max-width: 500px;
        background: var(--bg-surface);
        border-radius: 1rem 1rem 0 0; /* Bottom sheet style */
        padding: 1.5rem;
        box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.2);
        max-height: 80vh;
        overflow-y: auto;
    }

    .slide-up {
        animation: slideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    }

    @keyframes slideUp {
        from {
            transform: translateY(100%);
            opacity: 0;
        }
        to {
            transform: translateY(0);
            opacity: 1;
        }
    }

    .mobile-menu-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1.5rem;
    }

    .mobile-menu-header h3 {
        font-size: 1.1rem;
        font-weight: 600;
        margin: 0;
        color: var(--text-primary);
    }

    .close-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        padding: 0.5rem;
        cursor: pointer;
        border-radius: 50%;
    }

    .close-btn:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        width: 100%;
        padding: 0.875rem 1rem;
        background: transparent;
        border: none;
        color: var(--text-secondary);
        font-size: 0.95rem;
        font-weight: 500;
        cursor: pointer;
        border-radius: var(--radius-md);
        transition: all 0.2s;
        text-align: left;
        margin-bottom: 0.25rem;
    }

    .nav-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item.active {
        background: var(--color-primary);
        color: white;
    }

    .icon {
        display: flex;
        align-items: center;
        justify-content: center;
        opacity: 0.8;
    }

    .nav-item.active .icon {
        opacity: 1;
    }

    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg);
    }

    .card-header {
        padding: 1.5rem;
        border-bottom: 1px solid var(--border-color);
    }

    .card-title {
        font-size: 1.25rem;
        font-weight: 600;
        margin-bottom: 0.25rem;
        color: var(--text-primary);
    }

    .card-subtitle {
        color: var(--text-secondary);
        font-size: 0.9rem;
    }

    .settings-list {
        padding: 0 1.5rem;
    }

    .setting-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.5rem 0;
        border-bottom: 1px solid var(--border-color);
        gap: 1rem;
    }

    .setting-item:last-child {
        border-bottom: none;
    }

    @media (max-width: 640px) {
        .setting-item {
            flex-direction: column;
            align-items: flex-start;
            gap: 0.75rem;
        }

        .setting-info {
            width: 100%;
        }

        .setting-control {
            width: 100%;
            justify-content: flex-start;
        }

        .form-input,
        .select-wrapper {
            max-width: 100%;
            width: 100%;
        }

        /* Adjust Test Email Form for mobile */
        .test-email-form {
            flex-direction: column;
            align-items: stretch;
        }

        .test-email-form .form-input {
            max-width: 100%;
        }

        .card-footer {
            flex-direction: column-reverse;
        }

        .card-footer .btn {
            width: 100%;
            justify-content: center;
        }
    }

    .setting-info label {
        font-weight: 500;
        font-size: 1rem;
        color: var(--text-primary);
        display: block;
        margin-bottom: 0.25rem;
    }

    .description {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }

    .setting-control {
        min-width: 120px;
        display: flex;
        justify-content: flex-end;
    }

    .form-input {
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 0.6rem 0.85rem;
        border-radius: var(--radius-md);
        width: 100%;
        max-width: 280px;
        font-size: 0.9rem;
        transition: all 0.2s;
    }

    .form-input:focus {
        border-color: var(--color-primary);
        outline: none;
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    }

    .form-input:hover:not(:focus) {
        border-color: var(--text-secondary);
    }

    .select-wrapper {
        position: relative;
        min-width: 220px;
    }

    select.form-input {
        cursor: pointer;
        appearance: none;
        padding-right: 2rem;
        background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
        background-repeat: no-repeat;
        background-position: right 0.75rem center;
    }

    .toggle {
        position: relative;
        display: inline-block;
        width: 50px;
        height: 26px;
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
        transition: 0.4s;
        border-radius: 34px;
        border: 1px solid var(--border-color);
    }
    .slider:before {
        position: absolute;
        content: "";
        height: 18px;
        width: 18px;
        left: 3px;
        bottom: 3px;
        background-color: var(--text-secondary);
        transition: 0.4s;
        border-radius: 50%;
    }

    input:checked + .slider {
        background-color: var(--color-primary);
        border-color: var(--color-primary);
    }

    input:checked + .slider:before {
        transform: translateX(24px);
        background-color: white;
    }

    .card-footer {
        padding: 1.5rem;
        background: var(--bg-tertiary);
        border-top: 1px solid var(--border-color);
        display: flex;
        justify-content: flex-end;
        gap: 1rem;
        border-bottom-left-radius: var(--radius-lg);
        border-bottom-right-radius: var(--radius-lg);
    }

    .btn {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.6rem 1.2rem;
        border-radius: var(--radius-md);
        font-weight: 600;
        font-size: 0.9rem;
        cursor: pointer;
        border: none;
        transition: all 0.2s;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover:not(:disabled) {
        filter: brightness(1.1);
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

    .btn-secondary:hover:not(:disabled) {
        background: var(--bg-hover);
        color: var(--text-primary);
        border-color: var(--text-secondary);
    }

    .alert {
        padding: 1rem 1.25rem;
        margin-bottom: 1.5rem;
        border-radius: var(--radius-md);
        font-weight: 500;
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .alert-success {
        background: rgba(34, 197, 94, 0.1);
        border: 1px solid rgba(34, 197, 94, 0.2);
        color: #22c55e;
    }

    .alert-error {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.2);
        color: #ef4444;
    }

    .logo-preview {
        height: 40px;
        width: auto;
        object-fit: contain;
        border: 1px solid var(--border-color);
        border-radius: 4px;
        padding: 2px;
        background: var(--bg-tertiary);
        margin-right: 1rem;
    }
    .file-upload-wrapper {
        display: flex;
        align-items: center;
    }
    .file-input {
        font-size: 0.85rem;
        color: var(--text-secondary);
        border: none;
        background: transparent;
        padding: 0;
        width: auto;
    }
    .file-input::file-selector-button {
        margin-right: 1rem;
        padding: 0.5rem 1rem;
        border-radius: 4px;
        border: 1px solid var(--border-color);
        background: var(--bg-tertiary);
        color: var(--text-primary);
        cursor: pointer;
        transition: all 0.2s;
        font-family: var(--font-family);
    }
    .file-input::file-selector-button:hover {
        background: var(--bg-secondary);
    }

    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
    .fade-in {
        animation: fadeIn 0.4s ease-out;
    }

    /* Test Email Section */
    .test-email-section {
        padding: 1.5rem;
        border-top: 1px solid var(--border-color);
        background: var(--bg-tertiary);
    }
    .test-email-section h3 {
        font-size: 1rem;
        font-weight: 600;
        margin-bottom: 0.5rem;
        color: var(--text-primary);
    }
    .section-description {
        font-size: 0.875rem;
        color: var(--text-secondary);
        margin-bottom: 1rem;
    }
    .test-email-form {
        display: flex;
        gap: 1rem;
        align-items: center;
    }
    .test-email-form .form-input {
        flex: 1;
        max-width: 300px;
    }

    @media (max-width: 900px) {
        .layout-grid {
            grid-template-columns: 1fr;
            gap: 1.5rem;
        }

        .desktop-sidebar {
            display: none;
        }

        .mobile-fab {
            display: flex;
        }
    }
</style>

<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { user, isAdmin } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { goto } from "$app/navigation";
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

    // Options
    const localeOptions = [
        { value: "id-ID", label: "Bahasa Indonesia (ID)" },
        { value: "en-US", label: "English (US)" },
        { value: "en-GB", label: "English (UK)" },
        { value: "es-ES", label: "Español" },
        { value: "ja-JP", label: "日本語 (Japanese)" },
    ];

    const storageOptions = [
        { value: "local", label: "Local File System" },
        { value: "s3", label: "AWS S3 / Compatible" },
        { value: "r2", label: "Cloudflare R2" },
    ];

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
    };

    onMount(async () => {
        if (!$isAdmin) {
            goto("/dashboard");
            return;
        }
        await loadSettings();
    });

    async function loadSettings() {
        try {
            const [data, logo] = await Promise.all([
                api.settings.getAll(),
                api.settings.getLogo(),
            ]);
            logoBase64 = logo;

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

    async function handleFileUpload(e: Event) {
        const input = e.target as HTMLInputElement;
        if (!input.files || input.files.length === 0) return;

        const file = input.files[0];
        console.log(
            `[Upload] File selected: ${file.name}, Size: ${file.size} bytes`,
        );

        if (file.size > 2 * 1024 * 1024) {
            showMessage("error", "File size too large (Max 2MB)");
            return;
        }

        const reader = new FileReader();

        reader.onload = async () => {
            try {
                const result = reader.result as string;
                logoBase64 = result;

                const base64 = result.split(",")[1];

                console.log(`[Upload] Sending to backend...`);

                const path = await api.settings.uploadLogo(base64);
                console.log(`[Upload] Success! Path: ${path}`);

                localSettings["app_logo_path"] = path;
                localSettings = { ...localSettings };

                hasChanges = true;

                appSettings.updateSetting("app_logo_path", path);
                appLogo.set(result); // Update global logo store (Base64)

                showMessage("success", "Logo uploaded");
            } catch (error) {
                console.error("[Upload] Error:", error);
                showMessage("error", "Failed to upload logo: " + String(error));
            }
        };

        reader.onerror = (err) => {
            console.error("[Upload] Reader error:", err);
            showMessage("error", "Failed to read file");
        };

        reader.readAsDataURL(file);
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
        hasChanges = false;
    }

    function showMessage(type: "success" | "error", text: string) {
        message = { type, text };
        setTimeout(() => (message = { type: "", text: "" }), 3000);
    }

    function getInputType(key: string) {
        if (key === "default_locale") return "select-locale";
        if (key === "storage_provider") return "select-storage";
        if (key === "app_logo_path") return "file";
        if (
            key.includes("secret") ||
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
        if (key === "storage_provider") return true;

        const provider = localSettings["storage_provider"] || "local";

        if (key.startsWith("storage_local_") && provider !== "local")
            return false;
        if (key.startsWith("storage_s3_") && provider !== "s3") return false;
        if (key.startsWith("storage_r2_") && provider !== "r2") return false;

        return true;
    }

    function getLabel(key: string) {
        return key
            .replace("auth_", "")
            .replace("app_", "")
            .replace("storage_", "")
            .replace("s3_", "S3 ")
            .replace("r2_", "R2 ")
            .replace("local_", "Local ")
            .split("_")
            .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
            .join(" ");
    }
</script>

<div class="page-container fade-in">
    {#if message.text}
        <div class="alert alert-{message.type} slide-in">
            {message.text}
        </div>
    {/if}

    <div class="layout-grid">
        <aside class="sidebar card">
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
                        {cat.label}
                    </button>
                {/each}
            </nav>
        </aside>

        <main class="content">
            {#if loading}
                <div class="loading-state">Loading settings...</div>
            {:else}
                <div class="card section fade-in">
                    <div class="card-header">
                        <div class="header-text">
                            <h2 class="card-title">
                                {categories[activeTab].label}
                            </h2>
                            <p class="card-subtitle">
                                Manage settings for {categories[
                                    activeTab
                                ].label.toLowerCase()}.
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

                    <div class="card-footer">
                        <button
                            class="btn btn-secondary"
                            disabled={!hasChanges || saving}
                            on:click={discardChanges}
                        >
                            Reset
                        </button>
                        <button
                            class="btn btn-primary"
                            disabled={!hasChanges || saving}
                            on:click={saveChanges}
                        >
                            {#if saving}Saving...{:else}Save Changes{/if}
                        </button>
                    </div>
                </div>
            {/if}
        </main>
    </div>
</div>

<style>
    .page-container {
        padding: 1.5rem;
        max-width: 1400px;
        margin: 0 auto;
    }
    .header {
        margin-bottom: 2rem;
        display: none;
    } /* Hidden as per request */

    .layout-grid {
        display: grid;
        grid-template-columns: 260px 1fr;
        gap: 2rem;
        align-items: start;
    }

    .sidebar {
        padding: 1rem;
        position: sticky;
        top: 2rem;
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
        border-radius: var(--border-radius-sm);
        transition: all 0.2s;
        text-align: left;
        margin-bottom: 0.25rem;
    }
    .nav-item:hover {
        background: rgba(255, 255, 255, 0.05);
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
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
    }
    .card-header {
        padding: 1.5rem;
        border-bottom: 1px solid var(--border-color);
    }
    .card-title {
        font-size: 1.25rem;
        font-weight: 600;
        margin-bottom: 0.25rem;
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
    }
    .setting-item:last-child {
        border-bottom: none;
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
        background: var(--bg-primary);
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        padding: 0.5rem 0.75rem;
        border-radius: var(--border-radius-sm);
        width: 100%;
        max-width: 280px;
        font-size: 0.95rem;
        transition: border-color 0.2s;
    }
    .form-input:focus {
        border-color: var(--color-primary);
        outline: none;
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
        border-bottom-left-radius: var(--border-radius);
        border-bottom-right-radius: var(--border-radius);
    }
    .btn {
        padding: 0.6rem 1.2rem;
        border-radius: var(--border-radius-sm);
        font-weight: 500;
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
        background: var(--color-primary-hover);
    }
    .btn-primary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .btn-secondary {
        background: transparent;
        border: 1px solid var(--border-color);
        color: var(--text-secondary);
    }
    .btn-secondary:hover:not(:disabled) {
        background: var(--bg-primary);
        color: var(--text-primary);
    }

    .alert {
        padding: 1rem;
        margin-bottom: 1.5rem;
        border-radius: var(--border-radius-sm);
        font-weight: 500;
    }
    .alert-success {
        background: rgba(34, 197, 94, 0.1);
        border: 1px solid rgba(34, 197, 94, 0.2);
        color: #4ade80;
    }
    .alert-error {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.2);
        color: #f87171;
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
</style>

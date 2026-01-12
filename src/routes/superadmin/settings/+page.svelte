<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { isSuperAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import Icon from "$lib/components/Icon.svelte";
    import { toast } from "$lib/stores/toast";
    import type { Setting } from "$lib/api/client";

    let loading = true;
    let saving = false;
    let maintenanceMode = false;
    let maintenanceMessage = "";

    // Rate Limiting Settings
    let maxLoginAttempts = 5;
    let lockoutDurationMinutes = 15;
    let apiRateLimitPerMinute = 100;
    let enableIpBlocking = false;

    let hasChanges = false;

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

            maintenanceMode = settingsMap["maintenance_mode"] === "true";
            maintenanceMessage =
                settingsMap["maintenance_message"] ||
                "The system is currently under maintenance. Please try again later.";

            // Load rate limiting settings
            maxLoginAttempts = parseInt(
                settingsMap["max_login_attempts"] || "5",
            );
            lockoutDurationMinutes = parseInt(
                settingsMap["lockout_duration_minutes"] || "15",
            );
            apiRateLimitPerMinute = parseInt(
                settingsMap["api_rate_limit_per_minute"] || "100",
            );
            enableIpBlocking = settingsMap["enable_ip_blocking"] === "true";
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
            // Maintenance settings
            await api.settings.upsert(
                "maintenance_mode",
                maintenanceMode ? "true" : "false",
                "Global maintenance mode",
            );
            await api.settings.upsert(
                "maintenance_message",
                maintenanceMessage,
                "Message shown during maintenance",
            );

            // Rate limiting settings
            await api.settings.upsert(
                "max_login_attempts",
                maxLoginAttempts.toString(),
                "Maximum failed login attempts before lockout",
            );
            await api.settings.upsert(
                "lockout_duration_minutes",
                lockoutDurationMinutes.toString(),
                "Duration of account lockout in minutes",
            );
            await api.settings.upsert(
                "api_rate_limit_per_minute",
                apiRateLimitPerMinute.toString(),
                "Maximum API requests per minute per user",
            );
            await api.settings.upsert(
                "enable_ip_blocking",
                enableIpBlocking ? "true" : "false",
                "Enable automatic IP blocking for suspicious activity",
            );

            toast.success("Settings saved successfully");
            hasChanges = false;
        } catch (err) {
            console.error("Failed to save settings:", err);
            toast.error("Failed to save settings");
        } finally {
            saving = false;
        }
    }
</script>

<div class="page-header">
    <div class="header-left">
        <h1>Platform Settings</h1>
        <p class="subtitle">
            Configure global platform settings (Superadmin only)
        </p>
    </div>
</div>

<!-- Maintenance Mode Card -->
<div class="content-card">
    <div class="card-header">
        <h3>Maintenance Mode</h3>
    </div>
    <div class="card-body">
        {#if loading}
            <div class="loading-spinner">
                <div class="spinner"></div>
            </div>
        {:else}
            <div class="setting-row">
                <div class="setting-info">
                    <label class="setting-label">Enable Maintenance Mode</label>
                    <p class="setting-description">
                        When enabled, all users except superadmins will see a
                        maintenance page.
                    </p>
                </div>
                <label class="toggle">
                    <input
                        type="checkbox"
                        bind:checked={maintenanceMode}
                        on:change={handleChange}
                    />
                    <span class="slider"></span>
                </label>
            </div>

            <div class="setting-row">
                <div class="setting-info full-width">
                    <label class="setting-label" for="maintenance-message"
                        >Maintenance Message</label
                    >
                    <p class="setting-description">
                        Message displayed to users during maintenance.
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
        {/if}
    </div>
</div>

<!-- Security & Rate Limiting Card -->
<div class="content-card">
    <div class="card-header">
        <h3>Security & Rate Limiting</h3>
    </div>
    <div class="card-body">
        {#if loading}
            <div class="loading-spinner">
                <div class="spinner"></div>
            </div>
        {:else}
            <div class="setting-row">
                <div class="setting-info">
                    <label class="setting-label" for="max-login-attempts"
                        >Max Login Attempts</label
                    >
                    <p class="setting-description">
                        Number of failed login attempts before account lockout.
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
                    <label class="setting-label" for="lockout-duration"
                        >Lockout Duration</label
                    >
                    <p class="setting-description">
                        How long a user stays locked out after max failed
                        attempts.
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
                    <label class="setting-label" for="api-rate-limit"
                        >API Rate Limit</label
                    >
                    <p class="setting-description">
                        Maximum API requests allowed per minute per user.
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
                    <label class="setting-label">Enable IP Blocking</label>
                    <p class="setting-description">
                        Automatically block IP addresses with suspicious
                        activity.
                    </p>
                </div>
                <label class="toggle">
                    <input
                        type="checkbox"
                        bind:checked={enableIpBlocking}
                        on:change={handleChange}
                    />
                    <span class="slider"></span>
                </label>
            </div>
        {/if}
    </div>
</div>

<!-- Save Button -->
<div class="actions-footer">
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
            Save All Changes
        {/if}
    </button>
</div>

<style>
    .page-header {
        margin-bottom: 1.5rem;
    }

    h1 {
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

    .content-card {
        background: var(--bg-surface);
        border-radius: var(--radius-lg);
        border: 1px solid var(--border-color);
        box-shadow: var(--shadow-sm);
        overflow: hidden;
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

    .loading-spinner {
        display: flex;
        justify-content: center;
        padding: 2rem;
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

    .setting-row:last-of-type {
        border-bottom: none;
    }

    .setting-info {
        flex: 1;
    }

    .setting-info.full-width {
        width: 100%;
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
        margin: 0 0 0.75rem 0;
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

    .actions {
        margin-top: 1.5rem;
        display: flex;
        justify-content: flex-end;
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

    .actions-footer {
        margin-top: 1.5rem;
        display: flex;
        justify-content: flex-end;
        position: sticky;
        bottom: 0;
        padding: 1rem 0;
        background: linear-gradient(to top, var(--bg-app) 0%, transparent 100%);
    }

    .content-card {
        margin-bottom: 1.5rem;
    }
</style>

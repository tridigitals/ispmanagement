<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { isSuperAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import Icon from "$lib/components/Icon.svelte";
    import MobileFabMenu from "$lib/components/MobileFabMenu.svelte";
    import { toast } from "$lib/stores/toast";

    let loading = true;
    let saving = false;
    let activeTab = "general";

    // Data Models
    let maintenanceMode = false;
    let maintenanceMessage = "";

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

    let hasChanges = false;

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
            ];

            await Promise.all(updates);

            toast.success("Settings saved successfully");
            hasChanges = false;
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
                            <h3>Maintenance Mode</h3>
                        </div>
                        <div class="card-body">
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label class="setting-label"
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
                                    <label class="setting-label"
                                        >Allow Public Registration</label
                                    >
                                    <p class="setting-description">
                                        Allow new users to sign up freely.
                                    </p>
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={authAllowRegistration}
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label class="setting-label"
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
                                    <label class="setting-label"
                                        >Require Uppercase</label
                                    >
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
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
                                    <label class="setting-label"
                                        >Require Number</label
                                    >
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
                                        bind:checked={authPasswordRequireNumber}
                                        on:change={handleChange}
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
                            <div class="setting-row">
                                <div class="setting-info">
                                    <label class="setting-label"
                                        >Require Special Character</label
                                    >
                                </div>
                                <label class="toggle">
                                    <input
                                        type="checkbox"
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
                                    <label class="setting-label"
                                        >Enable IP Blocking</label
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
                                    />
                                    <span class="slider"></span>
                                </label>
                            </div>
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

    .content-card,
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

<script lang="ts">
    import { onMount } from "svelte";
    import { api } from "$lib/api/client";
    import { token, user } from "$lib/stores/auth";
    import { theme } from "$lib/stores/theme";
    import { appSettings } from "$lib/stores/settings";
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";
    import { t } from "svelte-i18n";
    import Icon from "$lib/components/ui/Icon.svelte";
    import MobileFabMenu from "$lib/components/ui/MobileFabMenu.svelte";
    import { getSlugFromDomain } from "$lib/utils/domain";
    import {
        preferences,
        loadPreferences,
        updatePreference,
        subscribePush,
        unsubscribePush,
        sendTestNotification,
        checkSubscription,
        pushEnabled,
    } from "$lib/stores/notifications";

    let activeTab = $state("general");
    let loading = $state(false);
    let message = $state<{ type: "" | "success" | "error"; text: string }>({
        type: "",
        text: "",
    });

    // User Data State (for editing)
    let profileData = $state({
        id: "",
        name: "",
        email: "",
        role: "",
    });

    // Password State
    let passwordData = $state({
        current: "",
        new: "",
        confirm: "",
    });

    // 2FA State
    let twoFactorData = $state({
        enabled: false,
        secret: "",
        qr: "",
        code: "",
        recoveryCodes: [] as string[],
        showSetup: false,
        showRecovery: false,
        disableCode: "",
    });

    // Visibility States
    let showCurrentPassword = $state(false);
    let showNewPassword = $state(false);
    let showConfirmPassword = $state(false);
    let isDesktop = $state(false);

    // Tenant prefix helper (supports custom domain mode)
    let domainSlug = $derived(getSlugFromDomain($page.url.hostname));
    let isCustomDomain = $derived(
        domainSlug && domainSlug === $user?.tenant_slug,
    );
    let tenantPrefix = $derived(
        $user?.tenant_slug && !isCustomDomain ? `/${$user.tenant_slug}` : "",
    );

    // UI Configuration
    let notificationCategories = $derived([
        {
            id: "system",
            icon: "server",
            label:
                $t("profile.notifications.categories.system.label") ||
                "System Updates",
            desc:
                $t("profile.notifications.categories.system.desc") ||
                "Maintenance & announcements",
        },
        {
            id: "team",
            icon: "users",
            label:
                $t("profile.notifications.categories.team.label") ||
                "Team Activity",
            desc:
                $t("profile.notifications.categories.team.desc") ||
                "Member changes & invites",
        },
        {
            // aligns with backend/category enum: 'payment'
            id: "payment",
            icon: "credit-card",
            label:
                $t("profile.notifications.categories.payment.label") ||
                "Billing",
            desc:
                $t("profile.notifications.categories.payment.desc") ||
                "Invoices & subscriptions",
        },
        {
            id: "security",
            icon: "shield",
            label:
                $t("profile.notifications.categories.security.label") ||
                "Security",
            desc:
                $t("profile.notifications.categories.security.desc") ||
                "Login alerts & password changes",
        },
    ]);

    let pushPermission = $state<"default" | "granted" | "denied">("default");

    let policy = $derived(
        $appSettings.auth || {
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_special: false,
        },
    );

    // Load initial data
    onMount(async () => {
        // Auth handled by layout

        // Don’t block first paint; load settings in background.
        void appSettings.init();

        // Check if running in Tauri Desktop
        isDesktop = !!(window as any).__TAURI_INTERNALS__;

        // Initialize profile data from store
        if ($user) {
            profileData = {
                id: $user.id,
                name: $user.name,
                email: $user.email,
                role: $user.role,
            };
            twoFactorData.enabled = $user.two_factor_enabled || false;
        }

        // Load notification preferences
        loadPreferences();
        checkSubscription();

        // Check URL for active tab or 2FA requirement
        const urlParams = new URLSearchParams(window.location.search);

        if (urlParams.get("2fa_required") === "true") {
            activeTab = "security";
            showMessage(
                "error",
                $t("profile.messages.twofa_required") ||
                    "Your organization requires Two-Factor Authentication. Please enable it to continue.",
            );
        } else {
            const tab = urlParams.get("tab");
            if (
                tab &&
                [
                    "general",
                    "security",
                    "preferences",
                    "notifications",
                ].includes(tab)
            ) {
                activeTab = tab;
            } else {
                // Restore last active tab if no explicit tab in URL.
                const saved = localStorage.getItem("profile.activeTab");
                if (
                    saved &&
                    [
                        "general",
                        "security",
                        "preferences",
                        "notifications",
                    ].includes(saved)
                ) {
                    activeTab = saved;
                }
            }
        }
    });

    $effect(() => {
        if (typeof window === "undefined") return;
        localStorage.setItem("profile.activeTab", activeTab);
    });

    // Helper to show messages
    function showMessage(type: "success" | "error", text: string) {
        message = { type, text };
        setTimeout(() => (message = { type: "", text: "" }), 4000);
    }

    // Save Profile
    async function saveProfile() {
        if (!$token) return;
        loading = true;
        try {
            await api.users.update(profileData.id, {
                name: profileData.name,
                email: profileData.email,
            });

            user.update((u) =>
                u
                    ? { ...u, name: profileData.name, email: profileData.email }
                    : null,
            );
            showMessage("success", $t("profile.messages.profile_updated"));
        } catch (error: any) {
            console.error(error);
            showMessage(
                "error",
                error.toString() || $t("profile.messages.update_failed"),
            );
        } finally {
            loading = false;
        }
    }

    function validatePassword(pwd: string): string | null {
        if (pwd.length < policy.password_min_length) {
            return $t("auth.validation.min_length", {
                values: { length: policy.password_min_length },
            });
        }
        if (policy.password_require_uppercase && !/[A-Z]/.test(pwd)) {
            return $t("auth.validation.require_uppercase");
        }
        if (policy.password_require_number && !/[0-9]/.test(pwd)) {
            return $t("auth.validation.require_number");
        }
        if (
            policy.password_require_special &&
            !/[!@#$%^&*()_+\-=[\]{}|;:',.<>?/`~]/.test(pwd)
        ) {
            return $t("auth.validation.require_special");
        }
        return null;
    }

    // Change Password
    async function changePassword() {
        if (!$token) return;

        if (passwordData.new !== passwordData.confirm) {
            showMessage("error", $t("profile.messages.password_mismatch"));
            return;
        }

        const policyError = validatePassword(passwordData.new);
        if (policyError) {
            showMessage("error", policyError);
            return;
        }

        loading = true;
        try {
            await api.auth.changePassword(
                $token,
                passwordData.current,
                passwordData.new,
            );

            showMessage("success", $t("profile.messages.password_changed"));
            passwordData = { current: "", new: "", confirm: "" };
        } catch (error: any) {
            console.error(error);
            showMessage(
                "error",
                error.toString() ||
                    $t("profile.messages.change_password_failed"),
            );
        } finally {
            loading = false;
        }
    }

    // 2FA Methods
    let setupMethod = $state<"totp" | "email">("totp");

    async function start2FA(method: "totp" | "email") {
        setupMethod = method;
        loading = true;
        try {
            if (method === "totp") {
                const { secret, qr } = await api.auth.enable2FA();
                twoFactorData.secret = secret;
                // Clean base64 string
                twoFactorData.qr = qr.replace(/\s/g, "");
            } else {
                await api.auth.requestEmail2FASetup();
            }
            twoFactorData.showSetup = true;
        } catch (error: any) {
            showMessage("error", error.toString());
        } finally {
            loading = false;
        }
    }

    async function verify2FA() {
        loading = true;
        try {
            if (setupMethod === "totp") {
                const { recovery_codes } = await api.auth.verify2FASetup(
                    twoFactorData.secret,
                    twoFactorData.code,
                );
                twoFactorData.recoveryCodes = recovery_codes;
            } else {
                await api.auth.verifyEmail2FASetup(twoFactorData.code);
                // Email 2FA doesn't have recovery codes usually, but we could generate them if we wanted.
                // For now, let's assume no recovery codes for email 2FA or fetch them separately?
                // Actually backend logic for email 2FA setup doesn't return recovery codes in my new impl.
                // So we just skip showing recovery codes for email setup.
                twoFactorData.recoveryCodes = [];
            }

            twoFactorData.enabled = true;
            twoFactorData.showSetup = false;

            // Only show recovery codes if we have them (TOTP flow)
            if (twoFactorData.recoveryCodes.length > 0) {
                twoFactorData.showRecovery = true;
            } else {
                showMessage("success", "Two-factor authentication enabled!");
            }

            user.update((u) =>
                u
                    ? {
                          ...u,
                          two_factor_enabled: true,
                          preferred_2fa_method: setupMethod,
                      }
                    : null,
            );
        } catch (error: any) {
            showMessage("error", error.toString());
        } finally {
            loading = false;
        }
    }

    async function disable2FA() {
        loading = true;
        try {
            await api.auth.disable2FA(twoFactorData.disableCode);
            twoFactorData.enabled = false;
            twoFactorData.disableCode = "";
            user.update((u) =>
                u ? { ...u, two_factor_enabled: false } : null,
            );
            showMessage("success", "Two-factor authentication disabled.");
        } catch (error: any) {
            showMessage("error", error.toString());
        } finally {
            loading = false;
        }
    }

    let disableOtpSending = $state(false);
    let disableOtpSent = $state(false);

    async function sendDisableEmailOtp() {
        disableOtpSending = true;
        try {
            await api.auth.request2FADisableCode();
            disableOtpSent = true;
            showMessage("success", "Verification code sent to your email.");
        } catch (error: any) {
            showMessage("error", error.toString());
        } finally {
            disableOtpSending = false;
        }
    }

    async function change2FAMethod(method: "totp" | "email") {
        if (!$user?.two_factor_enabled) return;
        loading = true;
        try {
            await api.auth.set2FAPreference(method);
            user.update((u) =>
                u ? { ...u, preferred_2fa_method: method } : null,
            );
            showMessage(
                "success",
                method === "totp"
                    ? $t("profile.messages.2fa_method_totp") ||
                          "2FA method changed to Authenticator App"
                    : $t("profile.messages.2fa_method_email") ||
                          "2FA method changed to Email",
            );
        } catch (error: any) {
            showMessage("error", error.toString());
        } finally {
            loading = false;
        }
    }

    // Get initials for avatar
    let initials = $derived(
        profileData.name
            ? profileData.name
                  .split(" ")
                  .map((n) => n[0])
                  .slice(0, 2)
                  .join("")
                  .toUpperCase()
            : "U",
    );

    let tabs = $derived([
        { id: "general", label: $t("profile.tabs.general"), icon: "profile" },
        { id: "security", label: $t("profile.tabs.security"), icon: "lock" },
        {
            id: "preferences",
            label: $t("profile.tabs.preferences"),
            icon: "settings",
        },
        {
            id: "notifications",
            label: $t("profile.tabs.notifications") || "Notifications",
            icon: "bell",
        },
    ]);
</script>

<div class="page-container fade-in">
    <div class="header-section">
        <h1 class="page-title">{$t("profile.title")}</h1>
        <p class="page-subtitle">{$t("profile.subtitle")}</p>
    </div>

    {#if message.text}
        <div class="alert alert-{message.type} slide-in">
            <Icon
                name={message.type === "success" ? "check" : "alert"}
                size={20}
            />
            <span>{message.text}</span>
        </div>
    {/if}

    <div class="layout-grid">
        <!-- Sidebar Navigation -->
        <aside class="sidebar">
            <div class="user-mini-profile">
                <div class="avatar-circle">{initials}</div>
                <div class="user-info">
                    <span class="name"
                        >{profileData.name ||
                            $t("profile.fallback.user") ||
                            "User"}</span
                    >
                    <span class="role"
                        >{profileData.role ||
                            $t("profile.fallback.member") ||
                            "Member"}</span
                    >
                </div>
            </div>

            <nav class="nav-menu">
                {#each tabs as tab}
                    <button
                        class="nav-item {activeTab === tab.id ? 'active' : ''}"
                        onclick={() => (activeTab = tab.id)}
                    >
                        <Icon name={tab.icon} size={18} />
                        <span>{tab.label}</span>
                        {#if activeTab === tab.id}
                            <div class="active-indicator"></div>
                        {/if}
                    </button>
                {/each}
            </nav>
        </aside>

        <!-- Mobile FAB & Menu -->
        <MobileFabMenu
            items={tabs}
            bind:activeTab
            title={$t("profile.title")}
        />

        <!-- Main Content Area -->
        <main class="content-area">
            {#if activeTab === "general"}
                <div class="card section fade-in-up">
                    <div class="card-header">
                        <div>
                            <h2 class="card-title">
                                {$t("profile.general.title")}
                            </h2>
                            <p class="card-subtitle">
                                {$t("profile.general.subtitle")}
                            </p>
                        </div>
                    </div>

                    <div class="profile-header-edit">
                        <div class="avatar-large-wrapper">
                            <div class="avatar-large">{initials}</div>
                            <button
                                class="avatar-edit-btn"
                                title={$t("profile.general.change_avatar") ||
                                    "Change Avatar"}
                            >
                                <Icon name="camera" size={16} />
                            </button>
                        </div>
                        <div class="profile-header-text">
                            <h3>
                                {profileData.name ||
                                    $t("profile.general.your_name") ||
                                    "Your Name"}
                            </h3>
                            <p>
                                {profileData.role ||
                                    $t("profile.general.role_label") ||
                                    "Role"}
                            </p>
                        </div>
                    </div>

                    <form
                        onsubmit={(e) => {
                            e.preventDefault();
                            saveProfile();
                        }}
                        class="settings-form"
                    >
                        <div class="form-group">
                            <label class="form-label" for="full-name"
                                >{$t("profile.general.display_name")}</label
                            >
                            <input
                                type="text"
                                id="full-name"
                                class="form-input"
                                placeholder={$t(
                                    "profile.general.display_name_placeholder",
                                )}
                                bind:value={profileData.name}
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="email"
                                >{$t("profile.general.email_address")}</label
                            >
                            <input
                                type="email"
                                id="email"
                                class="form-input"
                                placeholder={$t(
                                    "profile.general.email_placeholder",
                                )}
                                bind:value={profileData.email}
                            />
                        </div>

                        <div class="form-actions">
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled={loading}
                            >
                                {#if loading}
                                    <span class="spinner"></span>
                                    {$t("profile.general.saving")}
                                {:else}
                                    {$t("profile.general.save_button")}
                                {/if}
                            </button>
                        </div>
                    </form>
                </div>
            {/if}

            {#if activeTab === "security"}
                <!-- Existing Password Form -->
                <div class="card section fade-in-up">
                    <div class="card-header">
                        <h2 class="card-title">
                            {$t("profile.security.title")}
                        </h2>
                        <p class="card-subtitle">
                            {$t("profile.security.subtitle")}
                        </p>
                    </div>

                    <form
                        onsubmit={(e) => {
                            e.preventDefault();
                            changePassword();
                        }}
                        class="settings-form"
                    >
                        <div class="form-group">
                            <label class="form-label" for="current-pass"
                                >{$t(
                                    "profile.security.current_password",
                                )}</label
                            >
                            <div class="input-wrapper">
                                <input
                                    type={showCurrentPassword
                                        ? "text"
                                        : "password"}
                                    id="current-pass"
                                    class="form-input"
                                    placeholder={$t(
                                        "profile.security.password_placeholder",
                                    )}
                                    bind:value={passwordData.current}
                                />
                                <button
                                    type="button"
                                    class="toggle-password"
                                    onclick={() =>
                                        (showCurrentPassword =
                                            !showCurrentPassword)}
                                    tabindex="-1"
                                >
                                    <Icon
                                        name={showCurrentPassword
                                            ? "eye-off"
                                            : "eye"}
                                        size={18}
                                    />
                                </button>
                            </div>
                        </div>

                        <div class="grid-2">
                            <div class="form-group">
                                <label class="form-label" for="new-pass"
                                    >{$t(
                                        "profile.security.new_password",
                                    )}</label
                                >
                                <div class="input-wrapper">
                                    <input
                                        type={showNewPassword
                                            ? "text"
                                            : "password"}
                                        id="new-pass"
                                        class="form-input"
                                        placeholder={$t(
                                            "profile.security.password_placeholder",
                                        )}
                                        bind:value={passwordData.new}
                                    />
                                    <button
                                        type="button"
                                        class="toggle-password"
                                        onclick={() =>
                                            (showNewPassword =
                                                !showNewPassword)}
                                        tabindex="-1"
                                    >
                                        <Icon
                                            name={showNewPassword
                                                ? "eye-off"
                                                : "eye"}
                                            size={18}
                                        />
                                    </button>
                                </div>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="confirm-pass"
                                    >{$t(
                                        "profile.security.confirm_password",
                                    )}</label
                                >
                                <div class="input-wrapper">
                                    <input
                                        type={showConfirmPassword
                                            ? "text"
                                            : "password"}
                                        id="confirm-pass"
                                        class="form-input"
                                        placeholder={$t(
                                            "profile.security.password_placeholder",
                                        )}
                                        bind:value={passwordData.confirm}
                                    />
                                    <button
                                        type="button"
                                        class="toggle-password"
                                        onclick={() =>
                                            (showConfirmPassword =
                                                !showConfirmPassword)}
                                        tabindex="-1"
                                    >
                                        <Icon
                                            name={showConfirmPassword
                                                ? "eye-off"
                                                : "eye"}
                                            size={18}
                                        />
                                    </button>
                                </div>
                            </div>
                        </div>

                        <div class="password-requirements">
                            <p>{$t("profile.security.requirements_title")}</p>
                            <ul>
                                <li
                                    class={passwordData.new.length >=
                                    policy.password_min_length
                                        ? "valid"
                                        : ""}
                                >
                                    {$t("profile.security.req_length", {
                                        values: {
                                            length: policy.password_min_length,
                                        },
                                    })}
                                </li>
                                {#if policy.password_require_uppercase}
                                    <li
                                        class={/[A-Z]/.test(passwordData.new)
                                            ? "valid"
                                            : ""}
                                    >
                                        {$t("profile.security.req_uppercase")}
                                    </li>
                                {/if}
                                {#if policy.password_require_number}
                                    <li
                                        class={/[0-9]/.test(passwordData.new)
                                            ? "valid"
                                            : ""}
                                    >
                                        {$t("profile.security.req_number")}
                                    </li>
                                {/if}
                                {#if policy.password_require_special}
                                    <li
                                        class={/[!@#$%^&*()_+\-=[\]{}|;:',.<>?/`~]/.test(
                                            passwordData.new,
                                        )
                                            ? "valid"
                                            : ""}
                                    >
                                        {$t("profile.security.req_special")}
                                    </li>
                                {/if}
                            </ul>
                        </div>

                        <div class="form-actions">
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled={loading}
                            >
                                {loading
                                    ? $t("profile.security.updating")
                                    : $t("profile.security.update_button")}
                            </button>
                        </div>
                    </form>
                </div>

                <!-- 2FA Section -->
                <div class="card section fade-in-up" style="margin-top: 2rem;">
                    <div class="card-header">
                        <h2 class="card-title">
                            {$t("profile.security.twofa.title") ||
                                "Two-Factor Authentication"}
                        </h2>
                        <p class="card-subtitle">
                            {$t("profile.security.twofa.subtitle") ||
                                "Protect your account with an extra layer of security."}
                        </p>
                    </div>

                    <div class="setup-content">
                        {#if twoFactorData.enabled}
                            <div class="status-active">
                                <div class="status-icon">
                                    <Icon
                                        name="check"
                                        size={24}
                                        color="var(--success)"
                                    />
                                </div>
                                <div style="flex: 1">
                                    <h3>
                                        {$t(
                                            "profile.security.twofa.enabled_title",
                                        ) || "2FA is Enabled"}
                                    </h3>
                                    <p>
                                        {$t(
                                            "profile.security.twofa.enabled_desc",
                                        ) ||
                                            "Your account is secured with 2FA."}
                                    </p>

                                    <div
                                        class="method-selector"
                                        style="margin-top: 1rem;"
                                    >
                                        <h4
                                            style="font-size: 0.9rem; margin-bottom: 0.75rem; color: var(--text-secondary);"
                                        >
                                            Enabled Methods
                                        </h4>
                                        <div
                                            class="checkbox-group"
                                            style="display: flex; flex-direction: column; gap: 0.75rem;"
                                        >
                                            <!-- TOTP Method -->
                                            <label
                                                class="checkbox-card"
                                                style="padding: 0.75rem; border: 1px solid var(--border-color); border-radius: 8px; cursor: pointer; display: flex; align-items: center; gap: 0.75rem; transition: all 0.2s; background: var(--bg-primary);"
                                            >
                                                <input
                                                    type="checkbox"
                                                    checked={$user?.totp_enabled}
                                                    disabled={true}
                                                    style="accent-color: var(--color-primary);"
                                                />
                                                <Icon
                                                    name="smartphone"
                                                    size={18}
                                                />
                                                <span
                                                    style="flex: 1; font-weight: 500;"
                                                >
                                                    {$t(
                                                        "profile.security.twofa.methods.authenticator_app",
                                                    ) || "Authenticator App"}
                                                </span>
                                                {#if $user?.totp_enabled}
                                                    <span
                                                        style="font-size: 0.75rem; color: var(--success); background: rgba(34, 197, 94, 0.1); padding: 0.25rem 0.5rem; border-radius: 4px;"
                                                        >Enabled</span
                                                    >
                                                {:else}
                                                    <button
                                                        class="btn btn-sm btn-outline"
                                                        onclick={() =>
                                                            start2FA("totp")}
                                                        disabled={loading}
                                                    >
                                                        Enable
                                                    </button>
                                                {/if}
                                            </label>

                                            <!-- Email Method -->
                                            <label
                                                class="checkbox-card"
                                                style="padding: 0.75rem; border: 1px solid var(--border-color); border-radius: 8px; cursor: pointer; display: flex; align-items: center; gap: 0.75rem; transition: all 0.2s; background: var(--bg-primary);"
                                            >
                                                <input
                                                    type="checkbox"
                                                    checked={$user?.email_2fa_enabled}
                                                    disabled={true}
                                                    style="accent-color: var(--color-primary);"
                                                />
                                                <Icon name="mail" size={18} />
                                                <span
                                                    style="flex: 1; font-weight: 500;"
                                                >
                                                    {$t(
                                                        "profile.security.twofa.methods.email_verification",
                                                    ) || "Email Verification"}
                                                </span>
                                                {#if $user?.email_2fa_enabled}
                                                    <span
                                                        style="font-size: 0.75rem; color: var(--success); background: rgba(34, 197, 94, 0.1); padding: 0.25rem 0.5rem; border-radius: 4px;"
                                                        >Enabled</span
                                                    >
                                                {:else}
                                                    <button
                                                        class="btn btn-sm btn-outline"
                                                        onclick={() =>
                                                            start2FA("email")}
                                                        disabled={loading}
                                                    >
                                                        Enable
                                                    </button>
                                                {/if}
                                            </label>
                                        </div>

                                        <!-- Preferred Method Selector (only if both enabled) -->
                                        {#if $user?.totp_enabled && $user?.email_2fa_enabled}
                                            <h4
                                                style="font-size: 0.9rem; margin: 1rem 0 0.5rem; color: var(--text-secondary);"
                                            >
                                                Preferred Method
                                            </h4>
                                            <div
                                                class="radio-group"
                                                style="display: flex; gap: 1rem;"
                                            >
                                                <label
                                                    class="radio-card"
                                                    class:selected={$user?.preferred_2fa_method !==
                                                        "email"}
                                                    style="flex: 1; padding: 0.5rem 0.75rem; border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 0.5rem; transition: all 0.2s; background: var(--bg-primary);"
                                                >
                                                    <input
                                                        type="radio"
                                                        name="2fa_method"
                                                        value="totp"
                                                        checked={$user?.preferred_2fa_method !==
                                                            "email"}
                                                        onchange={() =>
                                                            change2FAMethod(
                                                                "totp",
                                                            )}
                                                        style="accent-color: var(--color-primary);"
                                                    />
                                                    <Icon
                                                        name="smartphone"
                                                        size={16}
                                                    />
                                                    <span
                                                        style="font-size: 0.85rem;"
                                                        >TOTP</span
                                                    >
                                                </label>
                                                <label
                                                    class="radio-card"
                                                    class:selected={$user?.preferred_2fa_method ===
                                                        "email"}
                                                    style="flex: 1; padding: 0.5rem 0.75rem; border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 0.5rem; transition: all 0.2s; background: var(--bg-primary);"
                                                >
                                                    <input
                                                        type="radio"
                                                        name="2fa_method"
                                                        value="email"
                                                        checked={$user?.preferred_2fa_method ===
                                                            "email"}
                                                        onchange={() =>
                                                            change2FAMethod(
                                                                "email",
                                                            )}
                                                        style="accent-color: var(--color-primary);"
                                                    />
                                                    <Icon
                                                        name="mail"
                                                        size={16}
                                                    />
                                                    <span
                                                        style="font-size: 0.85rem;"
                                                        >Email</span
                                                    >
                                                </label>
                                            </div>
                                        {/if}
                                    </div>
                                </div>
                            </div>

                            {#if twoFactorData.showRecovery}
                                <div class="recovery-codes-box">
                                    <h4>
                                        {$t(
                                            "profile.security.twofa.recovery_title",
                                        ) || "Save your Recovery Codes!"}
                                    </h4>
                                    <p>
                                        These codes are the ONLY way to access
                                        your account if you lose your phone.
                                    </p>
                                    <div class="code-grid">
                                        {#each twoFactorData.recoveryCodes as code}
                                            <div class="code-item">{code}</div>
                                        {/each}
                                    </div>
                                    <button
                                        class="btn btn-primary width-full"
                                        onclick={() =>
                                            (twoFactorData.showRecovery = false)}
                                        >{$t(
                                            "profile.security.twofa.recovery.saved_button",
                                        ) || "I have saved them"}</button
                                    >
                                </div>
                            {:else}
                                <div class="disable-box">
                                    <h4>
                                        {$t(
                                            "profile.security.twofa.disable_title",
                                        ) || "Disable 2FA"}
                                    </h4>
                                    <p>
                                        {#if $user?.preferred_2fa_method === "email"}
                                            To disable, please request a code
                                            and enter it below.
                                        {:else}
                                            To disable, please confirm by
                                            entering a code from your device.
                                        {/if}
                                    </p>

                                    {#if $user?.preferred_2fa_method === "email"}
                                        <button
                                            class="btn btn-outline btn-sm"
                                            style="margin-bottom: 1rem;"
                                            onclick={sendDisableEmailOtp}
                                            disabled={disableOtpSending}
                                        >
                                            <Icon name="mail" size={16} />
                                            <span>
                                                {disableOtpSending
                                                    ? "Sending..."
                                                    : disableOtpSent
                                                      ? "Resend Code"
                                                      : "Send Code"}
                                            </span>
                                        </button>
                                    {/if}

                                    <div class="form-group">
                                        <input
                                            type="text"
                                            class="form-input"
                                            bind:value={
                                                twoFactorData.disableCode
                                            }
                                            placeholder={$t(
                                                "profile.security.twofa.disable.placeholder",
                                            ) || "Enter authentication code"}
                                        />
                                    </div>
                                    <button
                                        class="btn btn-danger"
                                        onclick={disable2FA}
                                        disabled={twoFactorData.disableCode
                                            .length < 6 || loading}
                                    >
                                        {loading
                                            ? "Disabling..."
                                            : "Disable 2FA"}
                                    </button>
                                </div>
                            {/if}
                        {:else if !twoFactorData.showSetup}
                            <div class="empty-state-2fa">
                                <p>
                                    Add an extra layer of security to your
                                    account by requiring a code from your phone
                                    when logging in.
                                </p>
                                <h4
                                    style="margin: 1.5rem 0 1rem; color: var(--text-primary);"
                                >
                                    Choose Setup Method
                                </h4>
                                <div
                                    class="setup-method-actions"
                                    style="display: flex; gap: 1rem; justify-content: center;"
                                >
                                    <button
                                        class="btn btn-primary"
                                        onclick={() => start2FA("totp")}
                                        disabled={loading}
                                    >
                                        <div
                                            style="display: flex; align-items: center; gap: 0.5rem;"
                                        >
                                            <Icon name="smartphone" size={18} />
                                            <span>
                                                {$t(
                                                    "profile.security.twofa.methods.authenticator_app",
                                                ) || "Authenticator App"}
                                            </span>
                                        </div>
                                    </button>
                                    <button
                                        class="btn btn-secondary"
                                        onclick={() => start2FA("email")}
                                        disabled={loading}
                                        style="background: var(--bg-secondary); color: var(--text-primary); border: 1px solid var(--border-color);"
                                    >
                                        <div
                                            style="display: flex; align-items: center; gap: 0.5rem;"
                                        >
                                            <Icon name="mail" size={18} />
                                            <span>
                                                {$t(
                                                    "profile.security.twofa.methods.email_verification",
                                                ) || "Email Verification"}
                                            </span>
                                        </div>
                                    </button>
                                </div>
                            </div>
                        {:else}
                            <div class="setup-grid">
                                {#if setupMethod === "totp"}
                                    <div class="qr-section">
                                        <p class="step-label">
                                            1. Scan this QR code
                                        </p>
                                        <div class="qr-wrapper">
                                            <img
                                                src="data:image/png;base64,{twoFactorData.qr}"
                                                alt="QR Code"
                                                class="qr-img"
                                            />
                                        </div>
                                        <p class="secret-text">
                                            Key: {twoFactorData.secret}
                                        </p>
                                    </div>
                                    <div class="verify-section">
                                        <p class="step-label">
                                            2. Enter the code
                                        </p>
                                        <input
                                            type="text"
                                            class="form-input text-center text-lg"
                                            bind:value={twoFactorData.code}
                                            placeholder={$t(
                                                "common.otp_placeholder_spaced",
                                            ) || "000 000"}
                                            maxlength="6"
                                        />
                                        <div class="form-actions row">
                                            <button
                                                class="btn btn-outline"
                                                onclick={() =>
                                                    (twoFactorData.showSetup = false)}
                                                >{$t("common.cancel") ||
                                                    "Cancel"}</button
                                            >
                                            <button
                                                class="btn btn-primary"
                                                onclick={verify2FA}
                                                disabled={twoFactorData.code
                                                    .length < 6 || loading}
                                                >{loading
                                                    ? "Verifying..."
                                                    : "Activate"}</button
                                            >
                                        </div>
                                    </div>
                                {:else}
                                    <!-- Email Setup UI -->
                                    <div
                                        class="verify-section"
                                        style="max-width: 400px; margin: 0 auto; text-align: center; width: 100%;"
                                    >
                                        <div style="margin-bottom: 2rem;">
                                            <div
                                                style="width: 60px; height: 60px; background: var(--bg-secondary); border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto 1rem;"
                                            >
                                                <Icon
                                                    name="mail"
                                                    size={32}
                                                    color="var(--primary)"
                                                />
                                            </div>
                                            <h3 style="margin-bottom: 0.5rem;">
                                                Verify your Email
                                            </h3>
                                            <p
                                                style="color: var(--text-secondary);"
                                            >
                                                We sent a verification code to <strong
                                                    >{profileData.email}</strong
                                                >
                                            </p>
                                        </div>

                                        <div class="form-group">
                                            <label
                                                for="email-otp"
                                                class="form-label"
                                                style="text-align: left;"
                                                >{$t("auth.2fa.enter_code") ||
                                                    "Enter Verification Code"}</label
                                            >
                                            <input
                                                type="text"
                                                id="email-otp"
                                                class="form-input text-center text-lg"
                                                bind:value={twoFactorData.code}
                                                placeholder={$t(
                                                    "common.otp_placeholder",
                                                ) || "000000"}
                                                maxlength="6"
                                                style="letter-spacing: 0.5em;"
                                            />
                                        </div>

                                        <div
                                            class="form-actions row"
                                            style="margin-top: 1.5rem;"
                                        >
                                            <button
                                                class="btn btn-outline"
                                                onclick={() =>
                                                    (twoFactorData.showSetup = false)}
                                            >
                                                Cancel
                                            </button>
                                            <button
                                                class="btn btn-primary"
                                                onclick={verify2FA}
                                                disabled={twoFactorData.code
                                                    .length < 6 || loading}
                                            >
                                                {loading
                                                    ? "Verifying..."
                                                    : "Verify & Enable"}
                                            </button>
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                </div>
            {/if}

            {#if activeTab === "preferences"}
                <div class="card section fade-in-up">
                    <div class="card-header">
                        <h2 class="card-title">
                            {$t("profile.preferences.title")}
                        </h2>
                        <p class="card-subtitle">
                            {$t("profile.preferences.subtitle")}
                        </p>
                    </div>

                    <div class="setting-item">
                        <div class="setting-info">
                            <h3>{$t("profile.preferences.dark_mode")}</h3>
                            <p>{$t("profile.preferences.dark_mode_desc")}</p>
                        </div>
                        <label class="toggle">
                            <input
                                type="checkbox"
                                checked={$theme === "dark"}
                                onchange={() => theme.toggle()}
                            />
                            <span class="slider"></span>
                        </label>
                    </div>
                </div>
            {/if}

            {#if activeTab === "notifications"}
                <div class="card section fade-in-up">
                    <div class="notifications-header">
                        <div>
                            <h2 class="section-title">
                                {$t("profile.notifications.title") ||
                                    "Notification Preferences"}
                            </h2>
                            <p class="section-subtitle">
                                {$t("profile.notifications.subtitle") ||
                                    "Customize how and when you want to be notified."}
                            </p>
                        </div>
                        <div class="header-actions">
                            <button
                                class="btn btn-outline btn-sm"
                                onclick={() =>
                                    goto(`${tenantPrefix}/notifications`)}
                            >
                                <Icon name="bell" size={14} />
                                <span
                                    >{$t("profile.notifications.view_all") ||
                                        "View all"}</span
                                >
                            </button>
                            <button
                                class="btn btn-outline btn-sm"
                                onclick={sendTestNotification}
                            >
                                <Icon name="bell" size={14} />
                                <span
                                    >{$t("profile.notifications.test") ||
                                        "Test Notification"}</span
                                >
                            </button>
                        </div>
                    </div>

                    <!-- Push Notification Banner / Status Card -->
                    {#if !isDesktop}
                        {#if $pushEnabled}
                            <div class="push-banner active">
                                <div class="push-content">
                                    <div class="push-icon success">
                                        <Icon name="check" size={20} />
                                    </div>
                                    <div class="push-text">
                                        <h4>
                                            {$t(
                                                "profile.notifications.push.active_title",
                                            ) || "Push Notifications Active"}
                                        </h4>
                                        <p>
                                            {$t(
                                                "profile.notifications.push.active_desc",
                                            ) ||
                                                "You are subscribed to real-time updates on this device."}
                                        </p>
                                    </div>
                                </div>
                                <button
                                    class="btn btn-outline btn-sm"
                                    onclick={unsubscribePush}
                                >
                                    {$t("profile.notifications.push.disable") ||
                                        "Disable"}
                                </button>
                            </div>
                        {:else if pushPermission !== "granted" || !$pushEnabled}
                            <!-- Show enable banner if not enabled -->
                            <div class="push-banner">
                                <div class="push-content">
                                    <div class="push-icon">
                                        <Icon name="bell" size={20} />
                                    </div>
                                    <div class="push-text">
                                        <h4>
                                            {$t(
                                                "profile.notifications.push.enable_title",
                                            ) || "Enable Push Notifications"}
                                        </h4>
                                        <p>
                                            {$t(
                                                "profile.notifications.push.enable_desc",
                                            ) ||
                                                "Get real-time updates even when the app is closed."}
                                        </p>
                                    </div>
                                </div>
                                <button
                                    class="btn btn-dark btn-sm"
                                    onclick={async () => {
                                        await subscribePush();
                                        pushPermission =
                                            Notification.permission;
                                    }}
                                >
                                    {$t("profile.notifications.push.enable") ||
                                        "Enable Push"}
                                </button>
                            </div>
                        {/if}
                    {/if}

                    <div class="prefs-grid">
                        {#each notificationCategories as category}
                            <div class="pref-card">
                                <div class="pref-card-header">
                                    <div class="cat-icon {category.id}">
                                        <Icon name={category.icon} size={18} />
                                    </div>
                                    <div class="cat-info">
                                        <h3>{category.label}</h3>
                                        <p>{category.desc}</p>
                                    </div>
                                </div>

                                <div class="pref-channels">
                                    {#each ["in_app", "email", "push"] as channel}
                                        {@const pref = $preferences.find(
                                            (p) =>
                                                p.category === category.id &&
                                                p.channel === channel,
                                        )}
                                        {@const isDisabled =
                                            category.id === "security" &&
                                            channel === "email"}
                                        {@const isEnabled =
                                            category.id === "security" &&
                                            channel === "email"
                                                ? true
                                                : (pref?.enabled ??
                                                  channel === "in_app")}

                                        <label
                                            class="channel-row"
                                            class:disabled={isDisabled}
                                        >
                                            <div class="channel-info">
                                                <span class="channel-name">
                                                    {channel === "in_app"
                                                        ? $t(
                                                              "profile.notifications.channels.in_app",
                                                          ) || "In-App"
                                                        : channel === "email"
                                                          ? $t(
                                                                "profile.notifications.channels.email",
                                                            ) || "Email"
                                                          : $t(
                                                                "profile.notifications.channels.push",
                                                            ) || "Push"}
                                                </span>
                                                {#if isDisabled}
                                                    <span class="channel-note"
                                                        >{$t(
                                                            "profile.notifications.channels.required",
                                                        ) || "Required"}</span
                                                    >
                                                {/if}
                                            </div>

                                            <div class="switch">
                                                <input
                                                    type="checkbox"
                                                    checked={isEnabled}
                                                    disabled={isDisabled}
                                                    onchange={(e) =>
                                                        updatePreference(
                                                            channel,
                                                            category.id,
                                                            e.currentTarget
                                                                .checked,
                                                        )}
                                                />
                                                <span class="slider round"
                                                ></span>
                                            </div>
                                        </label>
                                    {/each}
                                </div>
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}
        </main>
    </div>
</div>

<style>
    /* Layout & Containers */
    .page-container {
        padding: 2rem;
        max-width: 1100px;
        margin: 0 auto;
        min-height: 100%;
    }

    .header-section {
        margin-bottom: 2.5rem;
    }

    .page-title {
        font-size: 1.875rem;
        font-weight: 700;
        color: var(--text-primary);
        letter-spacing: -0.025em;
    }

    .page-subtitle {
        color: var(--text-secondary);
        font-size: 1rem;
        margin-top: 0.5rem;
    }

    .layout-grid {
        display: grid;
        grid-template-columns: 260px 1fr;
        gap: 2.5rem;
        align-items: start;
    }

    @media (max-width: 900px) {
        .layout-grid {
            grid-template-columns: 1fr;
            gap: 1.5rem;
        }

        .sidebar {
            display: none;
        }
    }

    /* Sidebar */
    .sidebar {
        position: sticky;
        top: 2rem;
    }

    .user-mini-profile {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        margin-bottom: 1.5rem;
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-md);
    }

    .avatar-circle {
        width: 40px;
        height: 40px;
        border-radius: 50%;
        background: linear-gradient(
            135deg,
            var(--color-primary),
            var(--color-primary-hover)
        );
        color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: 600;
        font-size: 1rem;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    }

    .user-info {
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .user-info .name {
        font-weight: 600;
        font-size: 0.95rem;
        color: var(--text-primary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .user-info .role {
        font-size: 0.75rem;
        color: var(--text-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        font-weight: 500;
    }

    .nav-menu {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .nav-item {
        position: relative;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        width: 100%;
        padding: 0.75rem 1rem;
        background: transparent;
        border: none;
        color: var(--text-secondary);
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        border-radius: var(--radius-md);
        transition: all 0.2s ease;
        text-align: left;
    }

    .nav-item:hover {
        background: var(--bg-hover);
        color: var(--text-primary);
    }

    .nav-item.active {
        background: var(--bg-surface);
        color: var(--color-primary);
        font-weight: 600;
        box-shadow: var(--shadow-sm);
    }

    .active-indicator {
        position: absolute;
        left: 0;
        top: 50%;
        transform: translateY(-50%);
        width: 3px;
        height: 16px;
        background: var(--color-primary);
        border-radius: 0 2px 2px 0;
    }

    /* Content Cards */
    .card {
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-lg);
        padding: 2rem;
        box-shadow: var(--shadow-sm);
    }

    .card-header {
        margin-bottom: 2rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid var(--border-subtle);
    }

    .card-title {
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--text-primary);
        margin-bottom: 0.25rem;
    }

    .card-subtitle {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }

    /* Profile Header Edit */
    .profile-header-edit {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        margin-bottom: 2.5rem;
    }

    .avatar-large-wrapper {
        position: relative;
    }

    .avatar-large {
        width: 80px;
        height: 80px;
        border-radius: 50%;
        background: linear-gradient(
            135deg,
            var(--color-primary),
            var(--color-primary-hover)
        );
        color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 2rem;
        font-weight: 700;
        border: 4px solid var(--bg-surface);
        box-shadow: 0 0 0 1px var(--border-subtle);
    }

    .avatar-edit-btn {
        position: absolute;
        bottom: 0;
        right: 0;
        width: 28px;
        height: 28px;
        border-radius: 50%;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--text-secondary);
        cursor: pointer;
        transition: all 0.2s;
        box-shadow: var(--shadow-sm);
    }

    .avatar-edit-btn:hover {
        color: var(--color-primary);
        border-color: var(--color-primary);
    }

    /* Forms */
    .settings-form {
        max-width: 100%;
    }

    .form-group {
        margin-bottom: 1.5rem;
    }

    .form-label {
        display: block;
        margin-bottom: 0.5rem;
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-primary);
    }

    .form-input {
        width: 100%;
        padding: 0.75rem 1rem;
        background: var(
            --bg-app
        ); /* Slightly darker/lighter than surface to look indented */
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        color: var(--text-primary);
        font-size: 0.95rem;
        transition: all 0.2s;
    }

    .form-input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 2px var(--color-primary-subtle);
        background: var(--bg-surface);
    }

    .grid-2 {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1.5rem;
    }

    .form-actions {
        margin-top: 2.5rem;
        display: flex;
        justify-content: flex-end;
        border-top: 1px solid var(--border-subtle);
        padding-top: 1.5rem;
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
        transition: all 0.2s;
        border: none;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover:not(:disabled) {
        background: var(--color-primary-hover);
        transform: translateY(-1px);
    }

    .btn-primary:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    /* Settings Items */
    .setting-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.5rem 0;
        border-bottom: 1px solid var(--border-subtle);
    }

    .setting-item:last-child {
        border-bottom: none;
    }

    .setting-info h3 {
        font-size: 1rem;
        font-weight: 500;
        margin-bottom: 0.25rem;
        color: var(--text-primary);
    }

    .setting-info p {
        color: var(--text-secondary);
        font-size: 0.875rem;
    }

    /* Toggle Switch */
    .toggle {
        position: relative;
        display: inline-block;
        width: 44px;
        height: 24px;
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
        background-color: var(--bg-active);
        transition: 0.3s;
        border-radius: 24px;
    }

    .slider:before {
        position: absolute;
        content: "";
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
    }

    input:checked + .slider:before {
        transform: translateX(20px);
    }

    /* Alerts */
    .alert {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 1rem;
        margin-bottom: 2rem;
        border-radius: var(--radius-md);
        font-size: 0.9rem;
        font-weight: 500;
    }

    .alert-success {
        background: rgba(16, 185, 129, 0.1);
        border: 1px solid rgba(16, 185, 129, 0.2);
        color: var(--color-success);
    }

    .alert-error {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.2);
        color: var(--color-danger);
    }

    .password-requirements {
        margin-top: 1rem;
        padding: 1rem;
        background: var(--bg-app);
        border-radius: var(--radius-md);
        font-size: 0.85rem;
        color: var(--text-secondary);
    }

    .password-requirements ul {
        margin-top: 0.5rem;
        padding-left: 1.25rem;
    }

    .valid {
        color: var(--color-success);
    }

    /* Input Password Toggle */
    .input-wrapper {
        position: relative;
    }

    .toggle-password {
        position: absolute;
        right: 12px;
        top: 50%;
        transform: translateY(-50%);
        background: none;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        padding: 0;
        display: flex;
        align-items: center;
        transition: color 0.2s;
    }

    .toggle-password:hover {
        color: var(--text-primary);
    }

    .input-wrapper .form-input {
        padding-right: 40px;
    }

    .fade-in {
        animation: fadeIn 0.4s ease-out;
    }
    .fade-in-up {
        animation: fadeInUp 0.4s ease-out;
    }

    /* Spinner */
    .spinner {
        width: 1rem;
        height: 1rem;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-radius: 50%;
        border-top-color: white;
        animation: spin 0.8s linear infinite;
        display: inline-block;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    /* Responsive */
    /* Responsive */
    @media (max-width: 900px) {
        .layout-grid {
            grid-template-columns: 1fr;
            gap: 1.5rem;
        }

        .sidebar {
            display: none;
        }

        .grid-2 {
            grid-template-columns: 1fr;
        }

        .avatar-large {
            width: 60px;
            height: 60px;
            font-size: 1.5rem;
        }

        .card {
            padding: 1.5rem;
        }

        .form-actions {
            flex-direction: column-reverse;
            gap: 1rem;
        }

        .btn {
            width: 100%;
        }
    }
    /* Notification Preferences Redesign */
    .notifications-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 2rem;
        gap: 1rem;
    }

    .header-actions {
        display: flex;
        gap: 0.75rem;
        align-items: center;
        flex-wrap: wrap;
        justify-content: flex-end;
    }

    .section-title {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary);
        margin: 0 0 0.5rem 0;
    }

    .section-subtitle {
        color: var(--text-secondary);
        font-size: 0.95rem;
    }

    .push-banner {
        background: linear-gradient(135deg, var(--bg-surface), var(--bg-app));
        border: 1px solid var(--border-color);
        border-left: 4px solid var(--color-primary);
        border-radius: var(--radius-md);
        padding: 1.5rem;
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 2rem;
        box-shadow: var(--shadow-sm);
    }

    .push-content {
        display: flex;
        align-items: center;
        gap: 1.5rem;
    }

    .push-icon {
        width: 40px;
        height: 40px;
        border-radius: 50%;
        background: rgba(var(--primary-rgb, 59, 130, 246), 0.1);
        color: var(--color-primary);
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .push-text h4 {
        margin: 0 0 0.25rem 0;
        font-weight: 600;
        color: var(--text-primary);
    }

    .push-text p {
        margin: 0;
        font-size: 0.9rem;
        color: var(--text-secondary);
    }

    .prefs-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
        gap: 1.5rem;
    }

    .pref-card {
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-lg);
        overflow: hidden;
        transition:
            transform 0.2s,
            box-shadow 0.2s;
    }

    .pref-card:hover {
        border-color: var(--border-color);
        box-shadow: var(--shadow-md);
    }

    .pref-card-header {
        padding: 1.25rem;
        display: flex;
        align-items: center;
        gap: 1rem;
        background: var(--bg-app);
        border-bottom: 1px solid var(--border-subtle);
    }

    .cat-icon {
        width: 36px;
        height: 36px;
        border-radius: 10px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        color: var(--text-secondary);
    }

    .cat-icon.system {
        color: var(--color-info);
    }
    .cat-icon.team {
        color: var(--color-success);
    }
    .cat-icon.payment {
        color: var(--color-warning);
    }
    .cat-icon.security {
        color: var(--color-danger);
    }

    .cat-info h3 {
        margin: 0 0 0.2rem 0;
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .cat-info p {
        margin: 0;
        font-size: 0.8rem;
        color: var(--text-muted);
    }

    .pref-channels {
        padding: 0.5rem 0;
    }

    .channel-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.85rem 1.25rem;
        cursor: pointer;
        transition: background 0.1s;
    }

    .channel-row:hover:not(.disabled) {
        background: var(--bg-hover);
    }

    .channel-row.disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    .channel-info {
        display: flex;
        align-items: center;
        gap: 0.6rem;
        min-width: 0;
    }

    .channel-name {
        font-weight: 500;
        font-size: 0.9rem;
        color: var(--text-secondary);
    }

    .channel-note {
        font-size: 0.75rem;
        font-weight: 800;
        padding: 0.15rem 0.5rem;
        border-radius: 999px;
        border: 1px solid rgba(99, 102, 241, 0.35);
        background: rgba(99, 102, 241, 0.12);
        color: var(--text-primary);
        white-space: nowrap;
    }

    /* Modern Switch */
    .switch {
        position: relative;
        display: inline-block;
        width: 36px;
        height: 20px;
    }

    .switch input {
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
        background-color: var(--bg-active);
        transition: 0.3s;
        border: 1px solid var(--border-input);
    }

    .slider:before {
        position: absolute;
        content: "";
        height: 14px;
        width: 14px;
        left: 2px;
        bottom: 2px;
        background-color: white;
        transition: 0.3s;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
    }

    input:checked + .slider {
        background-color: var(--color-primary);
        border-color: var(--color-primary);
    }

    input:checked + .slider:before {
        transform: translateX(16px);
    }

    /* Round sliders */
    .slider.round {
        border-radius: 34px;
    }

    .slider.round:before {
        border-radius: 50%;
    }

    /* Button variants */
    .btn-dark {
        background: var(--text-primary);
        color: var(--bg-app);
        border: none;
    }
    .btn-dark:hover {
        opacity: 0.9;
    }
    .btn-outline {
        background: transparent;
        border: 1px solid var(--border-color);
        color: var(--text-primary);
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .btn-outline:hover {
        background: var(--bg-hover);
    }
    .btn-sm {
        padding: 0.5rem 1rem;
        font-size: 0.85rem;
    }

    .push-banner.active {
        border-left-color: var(--color-success);
        background: rgba(16, 185, 129, 0.05);
    }

    .push-icon.success {
        background: rgba(16, 185, 129, 0.1);
        color: var(--color-success);
    }

    @media (max-width: 600px) {
        .notifications-header {
            flex-direction: column;
            align-items: flex-start;
            margin-bottom: 1.25rem;
        }

        .header-actions {
            width: 100%;
            justify-content: flex-start;
        }

        .prefs-grid {
            grid-template-columns: 1fr;
        }

        .push-banner {
            flex-direction: column;
            align-items: flex-start;
            gap: 1rem;
        }

        .push-content {
            gap: 1rem;
        }
    }

    /* QR Code Styles */
    .qr-wrapper {
        background: white;
        padding: 1rem;
        border-radius: var(--radius-md);
        display: inline-block;
        margin: 1rem 0;
    }

    .qr-img {
        width: 200px;
        height: 200px;
        display: block;
    }

    .secret-text {
        font-family: monospace;
        background: var(--bg-secondary);
        padding: 0.5rem;
        border-radius: 4px;
        word-break: break-all;
        margin-top: 0.5rem;
    }
</style>

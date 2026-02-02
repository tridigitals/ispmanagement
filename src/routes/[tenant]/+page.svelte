<script lang="ts">
    import {
        login,
        isAuthenticated,
        isAdmin,
        user,
        isSuperAdmin,
    } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";
    import { get, derived } from "svelte/store";
    import { t } from "svelte-i18n";
    import Icon from "$lib/components/ui/Icon.svelte";

    import { api } from "$lib/api/client";

    let email = "";
    let password = "";
    let rememberMe = true;
    let error = "";
    let loading = false;
    let activeField = "";

    // 2FA State
    let step = "login"; // 'login' | '2fa-select' | '2fa-totp' | '2fa-email'
    let twoFactorCode = "";
    let tempToken = "";
    let available2FAMethods: string[] = [];
    let selected2FAMethod = "";
    let emailOtpSent = false;
    let emailOtpSending = false;

    let showPassword = false;

    $: appName = $appSettings.app_name || "Platform Core";
    $: appDescription =
        $appSettings.app_description ||
        "Enterprise-grade boilerplate built with Rust and SvelteKit. Secure, scalable, and lightweight.";

    // Derived store for registration allowed state - secure by default
    const allowRegistration = derived(
        appSettings,
        ($s) => $s.auth?.allow_registration === true,
    );

    onMount(async () => {
        await Promise.all([appSettings.init(), appLogo.init()]);

        if ($isAuthenticated) {
            const u = get(user);
            const slug = u?.tenant_slug;
            const urlTenant = $page.params.tenant;

            // Tenant Isolation Check
            if (
                urlTenant &&
                slug &&
                slug.toLowerCase() !== urlTenant.toLowerCase()
            ) {
                // Allow Superadmin to access any tenant workspace
                if (get(isSuperAdmin)) return;

                // Check for Tauri environment to allow auto-redirection
                // @ts-ignore
                const isTauri =
                    typeof window !== "undefined" &&
                    (window as any).__TAURI_INTERNALS__;

                if (isTauri) {
                    if (get(isAdmin)) {
                        goto(`/${slug}/admin`);
                    } else {
                        goto(`/${slug}/dashboard`);
                    }
                    return;
                }

                // Logged in user is in the wrong tenant workspace -> Logout
                await import("$lib/stores/auth").then((m) => m.logout());
                const msg =
                    $t("auth.login.error_wrong_tenant") ||
                    "You do not have access to this workspace.";
                error = `${msg} (Target: ${urlTenant}, Your Account: ${slug})`;
                return;
            }

            if (slug) {
                if (get(isAdmin)) {
                    goto(`/${slug}/admin`);
                } else {
                    goto(`/${slug}/dashboard`);
                }
            } else {
                goto("/dashboard");
            }
        }
    });

    // Resend countdown state
    let resendCountdown = 0;
    let resendInterval: ReturnType<typeof setInterval> | null = null;
    const RESEND_DELAY = 60; // seconds

    function startResendCountdown() {
        resendCountdown = RESEND_DELAY;
        if (resendInterval) clearInterval(resendInterval);
        resendInterval = setInterval(() => {
            resendCountdown--;
            if (resendCountdown <= 0) {
                if (resendInterval) clearInterval(resendInterval);
                resendInterval = null;
            }
        }, 1000);
    }

    async function sendEmailOtp(isResend = false) {
        emailOtpSending = true;
        error = "";
        try {
            await api.auth.requestEmailOtp(tempToken);
            emailOtpSent = true;
            startResendCountdown();

            // Show toast on resend
            if (isResend) {
                const { toast } = await import("svelte-sonner");
                toast.success(
                    $t("auth.2fa.code_resent") || "Verification code sent!",
                );
            }
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            emailOtpSending = false;
        }
    }

    async function selectMethod(method: string) {
        selected2FAMethod = method;
        twoFactorCode = "";
        error = "";

        if (method === "email") {
            step = "2fa-email";
            await sendEmailOtp();
        } else {
            step = "2fa-totp";
        }
    }

    let trustDevice = false;

    async function handle2FAVerify() {
        if (!twoFactorCode || twoFactorCode.length < 6) return;
        error = "";
        loading = true;

        try {
            let response;
            if (selected2FAMethod === "email") {
                response = await api.auth.verifyEmailOtp(
                    tempToken,
                    twoFactorCode,
                    trustDevice,
                );
            } else {
                response = await api.auth.verifyLogin2FA(
                    tempToken,
                    twoFactorCode,
                    trustDevice,
                );
            }

            if (response.token && response.user) {
                // Domain Validation: Prevent login on wrong domain (Web only)
                const customDomain =
                    response.tenant?.custom_domain ||
                    response.user.tenant_custom_domain;
                const currentHost = window.location.hostname;
                // @ts-ignore
                const isTauri =
                    typeof window !== "undefined" &&
                    (window as any).__TAURI_INTERNALS__;
                const isLocalhost =
                    currentHost === "localhost" || currentHost === "127.0.0.1";
                const mainDomain = $appSettings.auth?.main_domain;
                const isMainDomain = mainDomain && currentHost === mainDomain;

                if (
                    !isTauri &&
                    !isLocalhost &&
                    !isMainDomain &&
                    customDomain &&
                    currentHost !== customDomain &&
                    !response.user.is_super_admin
                ) {
                    error = "Invalid login credentials or unauthorized domain.";
                    loading = false;
                    return;
                }

                // Store auth data
                const { setAuthData } = await import("$lib/stores/auth");
                setAuthData(response.token, response.user, rememberMe);
                redirectAfterLogin(response.user, response.tenant);
            }
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }

    async function redirectAfterLogin(u: any, t?: any) {
        const slug = u?.tenant_slug;
        const customDomain = t?.custom_domain || u?.tenant_custom_domain;
        const currentHost = window.location.hostname;
        // @ts-ignore
        const isTauri =
            typeof window !== "undefined" &&
            (window as any).__TAURI_INTERNALS__;
        const isLocalhost =
            currentHost === "localhost" || currentHost === "127.0.0.1";
        const mainDomain = $appSettings.auth?.main_domain;
        const isMainDomain = mainDomain && currentHost === mainDomain;

        // Domain Validation: Prevent login on wrong domain (Web only) - skip localhost and main domain
        if (
            !isTauri &&
            !isLocalhost &&
            !isMainDomain &&
            customDomain &&
            currentHost !== customDomain &&
            !u.is_super_admin
        ) {
            const { logout } = await import("$lib/stores/auth");
            logout();
            error = "Invalid login credentials or unauthorized domain.";
            return;
        }

        if (slug) {
            if ($page.url.hostname.includes(slug)) {
                if (u.role === "admin") {
                    goto(`/admin`);
                } else {
                    goto(`/dashboard`);
                }
            } else {
                if (u.role === "admin") {
                    goto(`/${slug}/admin`);
                }
            } // This closing brace was missing in the original search
        } else {
            goto("/dashboard");
        }
    }

    async function handleSubmit(e: Event) {
        e.preventDefault();
        error = "";
        loading = true;

        try {
            const response = await login(email, password, rememberMe);

            // Check for 2FA requirement FIRST
            if (response.requires_2fa) {
                tempToken = response.temp_token || "";
                available2FAMethods = response.available_2fa_methods || [
                    "totp",
                ];

                if (available2FAMethods.length === 1) {
                    selected2FAMethod = available2FAMethods[0];
                    step =
                        available2FAMethods[0] === "email"
                            ? "2fa-email"
                            : "2fa-totp";

                    if (available2FAMethods[0] === "email") {
                        await sendEmailOtp();
                    }
                } else {
                    step = "2fa-select";
                }
                loading = false;
                return;
            }

            // Domain Validation: Prevent login on wrong domain (Web only)
            const customDomain =
                response.tenant?.custom_domain ||
                response.user.tenant_custom_domain;
            const currentHost = window.location.hostname;
            // @ts-ignore
            const isTauri =
                typeof window !== "undefined" &&
                (window as any).__TAURI_INTERNALS__;

            if (
                !isTauri &&
                customDomain &&
                currentHost !== customDomain &&
                !response.user.is_super_admin
            ) {
                // Logout immediately
                await import("$lib/stores/auth").then((m) => m.logout());
                error = "Invalid login credentials or unauthorized domain.";
                loading = false;
                return;
            }

            const userSlug = response.user?.tenant_slug;
            const urlTenant = $page.params.tenant;

            // Tenant Isolation Check
            // If we are on a tenant-specific route, ensure the user belongs to this tenant
            if (
                urlTenant &&
                userSlug &&
                urlTenant.toLowerCase() !== userSlug.toLowerCase()
            ) {
                // Allow Superadmin to login to any tenant workspace
                if (response.user.is_super_admin) {
                    // Proceed
                } else {
                    // Check for Tauri environment to allow auto-redirection
                    // @ts-ignore
                    const isTauri =
                        typeof window !== "undefined" &&
                        (window as any).__TAURI_INTERNALS__;

                    if (isTauri) {
                        // Allow redirection
                    } else {
                        // Logout immediately if mismatch (WEB ONLY)
                        await import("$lib/stores/auth").then((m) =>
                            m.logout(),
                        );

                        const msg =
                            $t("auth.login.error_wrong_tenant") ||
                            "You do not have access to this workspace. Please login at your own workspace URL.";
                        throw new Error(
                            `${msg} (Target: ${urlTenant}, Your Account: ${userSlug})`,
                        );
                    }
                }
            }

            const slug = userSlug;

            if (slug) {
                redirectAfterLogin(response.user, response.tenant);
            } else {
                goto("/dashboard"); // Fallback
            }
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            loading = false;
        } finally {
            // loading = false; // handled in catch or goto
        }
    }
</script>

<div class="login-container">
    <div class="form-section">
        <div class="form-wrapper">
            <div class="form-header">
                <h2>{$t("auth.login.title")}</h2>
                <p>{$t("auth.login.subtitle")}</p>
            </div>

            {#if error}
                <div class="alert error" in:fly={{ y: -10 }}>
                    {error}
                </div>
            {/if}

            {#if step === "login"}
                <form on:submit={handleSubmit}>
                    <div
                        class="input-group"
                        class:focus={activeField === "email"}
                    >
                        <label for="email">{$t("auth.login.email_label")}</label
                        >
                        <div class="field">
                            <span class="icon"
                                ><Icon name="mail" size={18} /></span
                            >
                            <input
                                type="email"
                                id="email"
                                bind:value={email}
                                on:focus={() => (activeField = "email")}
                                on:blur={() => (activeField = "")}
                                placeholder={$t("auth.login.email_placeholder")}
                                required
                            />
                        </div>
                    </div>

                    <div
                        class="input-group"
                        class:focus={activeField === "password"}
                    >
                        <label for="password"
                            >{$t("auth.login.password_label")}</label
                        >
                        <div class="field">
                            <span class="icon"
                                ><Icon name="lock" size={18} /></span
                            >
                            <input
                                type={showPassword ? "text" : "password"}
                                id="password"
                                bind:value={password}
                                on:focus={() => (activeField = "password")}
                                on:blur={() => (activeField = "")}
                                placeholder={$t(
                                    "auth.login.password_placeholder",
                                )}
                                required
                                class="password-input"
                            />
                            <button
                                type="button"
                                class="toggle-password"
                                on:click={() => (showPassword = !showPassword)}
                                tabindex="-1"
                            >
                                <Icon
                                    name={showPassword ? "eye-off" : "eye"}
                                    size={18}
                                />
                            </button>
                        </div>
                    </div>

                    <div class="form-utils">
                        <label class="checkbox">
                            <input type="checkbox" bind:checked={rememberMe} />
                            <span class="checkmark"></span>
                            <span>{$t("auth.login.remember_me")}</span>
                        </label>
                        <a href="/forgot-password"
                            >{$t("auth.login.forgot_password")}</a
                        >
                    </div>

                    <button
                        type="submit"
                        class="btn-primary"
                        disabled={loading}
                    >
                        {#if loading}
                            <div class="spinner"></div>
                        {:else}
                            {$t("auth.login.submit_button")}
                        {/if}
                    </button>
                </form>

                {#if $allowRegistration}
                    <p class="footer-text">
                        {$t("auth.login.footer_text")}
                        <a href="/register">{$t("auth.login.register_link")}</a>
                    </p>
                {/if}
            {:else if step === "2fa-select"}
                <!-- 2FA Method Selection -->
                <div class="twofa-section" in:fade>
                    <h3>
                        {$t("auth.2fa.select_method") ||
                            "Select Verification Method"}
                    </h3>
                    <p>
                        {$t("auth.2fa.select_method_desc") ||
                            "Choose how you'd like to verify your identity"}
                    </p>
                    <div class="twofa-options">
                        {#if available2FAMethods.includes("totp")}
                            <button
                                class="twofa-option"
                                on:click={() => selectMethod("totp")}
                            >
                                <Icon name="smartphone" size={24} />
                                <span
                                    >{$t("auth.2fa.use_app") ||
                                        "Authenticator App"}</span
                                >
                            </button>
                        {/if}
                        {#if available2FAMethods.includes("email")}
                            <button
                                class="twofa-option"
                                on:click={() => selectMethod("email")}
                            >
                                <Icon name="mail" size={24} />
                                <span
                                    >{$t("auth.2fa.use_email") ||
                                        "Email Code"}</span
                                >
                            </button>
                        {/if}
                    </div>
                    <button
                        class="btn-link"
                        on:click={() => {
                            step = "login";
                            error = "";
                        }}
                    >
                        {$t("common.back") || "Back to Login"}
                    </button>
                </div>
            {:else if step === "2fa-totp"}
                <!-- TOTP Verification -->
                <div class="twofa-section" in:fade>
                    <h3>
                        {$t("auth.2fa.enter_code") || "Enter Verification Code"}
                    </h3>
                    <p>
                        {$t("auth.2fa.totp_desc") ||
                            "Enter the 6-digit code from your authenticator app"}
                    </p>
                    <div class="input-group">
                        <input
                            type="text"
                            bind:value={twoFactorCode}
                            maxlength="6"
                            placeholder={$t("common.otp_placeholder") ||
                                "000000"}
                            class="otp-input"
                            autocomplete="one-time-code"
                        />
                    </div>
                    <label
                        class="checkbox"
                        style="margin-bottom: 1rem; justify-content: center;"
                    >
                        <input type="checkbox" bind:checked={trustDevice} />
                        <span class="checkmark"></span>
                        <span
                            >{$t("auth.2fa.trust_device") ||
                                "Trust this device for 30 days"}</span
                        >
                    </label>
                    <button
                        class="btn-primary"
                        disabled={loading || twoFactorCode.length < 6}
                        on:click={handle2FAVerify}
                    >
                        {#if loading}
                            <div class="spinner"></div>
                        {:else}
                            {$t("auth.2fa.verify") || "Verify"}
                        {/if}
                    </button>
                    <button
                        class="btn-link"
                        on:click={() => {
                            step =
                                available2FAMethods.length > 1
                                    ? "2fa-select"
                                    : "login";
                            error = "";
                        }}
                    >
                        {$t("common.back") || "Back"}
                    </button>
                </div>
            {:else if step === "2fa-email"}
                <!-- Email OTP Verification -->
                <div class="twofa-section" in:fade>
                    <h3>
                        {$t("auth.2fa.enter_email_code") || "Enter Email Code"}
                    </h3>
                    <p>
                        {#if emailOtpSent}
                            {$t("auth.2fa.email_sent") ||
                                "A verification code has been sent to your email."}
                        {:else if emailOtpSending}
                            {$t("auth.2fa.sending_email") ||
                                "Sending verification code..."}
                        {:else}
                            {$t("auth.2fa.email_desc") ||
                                "We'll send a verification code to your email."}
                        {/if}
                    </p>
                    <div class="input-group">
                        <input
                            type="text"
                            bind:value={twoFactorCode}
                            maxlength="6"
                            placeholder={$t("common.otp_placeholder") ||
                                "000000"}
                            class="otp-input"
                            autocomplete="one-time-code"
                        />
                    </div>
                    <label
                        class="checkbox"
                        style="margin-bottom: 1rem; justify-content: center;"
                    >
                        <input type="checkbox" bind:checked={trustDevice} />
                        <span class="checkmark"></span>
                        <span
                            >{$t("auth.2fa.trust_device") ||
                                "Trust this device for 30 days"}</span
                        >
                    </label>
                    <button
                        class="btn-primary"
                        disabled={loading || twoFactorCode.length < 6}
                        on:click={handle2FAVerify}
                    >
                        {#if loading}
                            <div class="spinner"></div>
                        {:else}
                            {$t("auth.2fa.verify") || "Verify"}
                        {/if}
                    </button>
                    {#if emailOtpSent}
                        <button
                            class="btn-link resend"
                            disabled={emailOtpSending || resendCountdown > 0}
                            on:click={() => sendEmailOtp(true)}
                        >
                            {#if resendCountdown > 0}
                                {$t("auth.2fa.resend_code") || "Resend Code"} ({resendCountdown}s)
                            {:else}
                                {$t("auth.2fa.resend_code") || "Resend Code"}
                            {/if}
                        </button>
                    {/if}
                    <button
                        class="btn-link"
                        on:click={() => {
                            step =
                                available2FAMethods.length > 1
                                    ? "2fa-select"
                                    : "login";
                            error = "";
                        }}
                    >
                        {$t("common.back") || "Back"}
                    </button>
                </div>
            {/if}
        </div>
    </div>
</div>

<style>
    .login-container {
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
        background: var(--bg-primary);
    }

    .form-section {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: clamp(1.25rem, 4vw, 2rem);
        width: 100%;
    }

    .form-wrapper {
        width: 100%;
        max-width: 480px;
        background: var(--bg-surface);
        padding: clamp(1.5rem, 4vw, 2.5rem);
        border-radius: var(--radius-lg);
        border: 1px solid var(--border-color);
        box-shadow: var(--shadow-md);
    }

    .form-header {
        margin-bottom: 2rem;
        text-align: center;
    }

    .form-header h2 {
        font-size: 1.75rem;
        font-weight: 700;
        color: var(--text-primary);
    }

    .form-header p {
        color: var(--text-secondary);
        margin-top: 0.5rem;
    }

    .input-group {
        margin-bottom: 1.5rem;
    }

    .input-group label {
        display: block;
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-secondary);
        margin-bottom: 0.5rem;
    }

    .field {
        position: relative;
        display: flex;
        align-items: center;
    }

    .field .icon {
        position: absolute;
        left: 1rem;
        color: var(--text-muted);
        transition: color 0.2s;
    }

    .field input {
        width: 100%;
        padding: 0.75rem 1rem 0.75rem 3rem;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        color: var(--text-primary);
        font-size: 1rem;
        transition: all 0.2s;
    }

    .field input.password-input {
        padding-right: 40px;
    }

    .toggle-password {
        position: absolute;
        right: 10px;
        background: none;
        border: none;
        color: var(--text-muted);
        cursor: pointer;
        padding: 0;
        display: flex;
        align-items: center;
        transition: color 0.2s;
        z-index: 2;
    }

    .toggle-password:hover {
        color: var(--color-primary);
    }

    .input-group.focus .field input {
        border-color: var(--color-primary);
        background: var(--bg-primary);
    }

    .input-group.focus .field .icon {
        color: var(--color-primary);
    }

    .form-utils {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2rem;
        font-size: 0.85rem;
    }

    .checkbox {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
        color: var(--text-secondary);
    }

    .checkbox input {
        display: none;
    }

    .checkmark {
        width: 16px;
        height: 16px;
        border: 1px solid var(--border-color);
        border-radius: 4px;
        position: relative;
    }

    .checkbox input:checked + .checkmark {
        background: var(--color-primary);
        border-color: var(--color-primary);
    }

    .checkbox input:checked + .checkmark::after {
        content: "";
        position: absolute;
        left: 5px;
        top: 2px;
        width: 3px;
        height: 7px;
        border: solid white;
        border-width: 0 2px 2px 0;
        transform: rotate(45deg);
    }

    .form-utils a {
        color: var(--color-primary-light);
        text-decoration: none;
        font-weight: 600;
    }

    .btn-primary {
        width: 100%;
        padding: 0.75rem;
        background: var(--color-primary);
        color: white;
        border: none;
        border-radius: 8px;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        transition: opacity 0.2s;
        display: flex;
        justify-content: center;
    }

    .btn-primary:hover {
        opacity: 0.9;
    }
    .btn-primary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .footer-text {
        text-align: center;
        margin-top: 2rem;
        font-size: 0.9rem;
        color: var(--text-secondary);
    }

    .footer-text a {
        color: var(--text-primary);
        font-weight: 600;
        text-decoration: none;
    }

    .alert {
        padding: 0.75rem;
        border-radius: 8px;
        margin-bottom: 1.5rem;
        font-size: 0.85rem;
        text-align: center;
    }

    .alert.error {
        background: rgba(239, 68, 68, 0.1);
        color: #fca5a5;
        border: 1px solid rgba(239, 68, 68, 0.2);
    }

    .spinner {
        width: 20px;
        height: 20px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    /* 2FA Styles */
    .twofa-section {
        text-align: center;
    }

    .twofa-section h3 {
        font-size: 1.25rem;
        margin-bottom: 0.5rem;
        color: var(--text-primary);
    }

    .twofa-section p {
        color: var(--text-secondary);
        margin-bottom: 1.5rem;
        font-size: 0.9rem;
    }

    .twofa-options {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        margin-bottom: 1.5rem;
    }

    .twofa-option {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        cursor: pointer;
        transition: all 0.2s;
        color: var(--text-primary);
    }

    .twofa-option:hover {
        border-color: var(--color-primary);
        background: rgba(var(--color-primary-rgb), 0.05);
    }

    .otp-input {
        text-align: center;
        font-size: 1.5rem;
        letter-spacing: 0.5rem;
        padding: 1rem;
        width: 100%;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-md);
        color: var(--text-primary);
        margin-bottom: 1rem;
    }

    .otp-input:focus {
        outline: none;
        border-color: var(--color-primary);
    }

    .btn-link {
        background: none;
        border: none;
        color: var(--color-primary);
        cursor: pointer;
        font-size: 0.9rem;
        padding: 0.5rem;
        margin-top: 1rem;
        display: block;
        width: 100%;
        text-align: center;
    }

    .btn-link:hover {
        text-decoration: underline;
    }

    .btn-link.resend {
        margin-top: 0.5rem;
    }

    .btn-link:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    @media (max-width: 480px) {
        .form-wrapper {
            padding: 1.25rem;
        }

        .form-header h2 {
            font-size: 1.4rem;
        }
    }
</style>

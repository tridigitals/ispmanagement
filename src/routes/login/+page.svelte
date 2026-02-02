<script lang="ts">
    import {
        login,
        isAuthenticated,
        isAdmin,
        user,
        token,
    } from "$lib/stores/auth";
    import { api } from "$lib/api/client";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";
    import { get, derived } from "svelte/store";
    import { t } from "svelte-i18n";
    import Icon from "$lib/components/ui/Icon.svelte";

    let email = "";
    let password = "";
    let rememberMe = true;
    let error = "";
    let loading = false;
    let activeField = "";

    // 2FA State
    let step = "login"; // 'login' | '2fa-select' | '2fa-totp' | '2fa-email'
    // State to track 2FA input
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
            // Re-use the main redirection logic
            // Note: We don't have the full tenant object here, but we now have u.tenant_custom_domain
            // from the updated User model.
            redirectUser(u, undefined);
        }
    });

    async function redirectUser(u: any, t?: any) {
        const slug = u?.tenant_slug;
        // Prefer tenant object, fallback to user's enriched property
        const customDomain = t?.custom_domain || u?.tenant_custom_domain;
        const currentHost = window.location.hostname;
        // @ts-ignore
        const isTauri =
            typeof window !== "undefined" &&
            (window as any).__TAURI_INTERNALS__;

        // 1. Super Admin: Can login anywhere
        if (u.is_super_admin) {
            // If logged in on a tenant domain/subdomain, go to tenant admin
            if (slug && currentHost.includes(slug)) {
                goto(`/${slug}/admin`);
            } else {
                // Otherwise go to superadmin dashboard
                goto("/superadmin");
            }
            return;
        }

        // 2. Tenant User with Custom Domain
        // Skip domain check for localhost (development), main domain, or if no custom domain set
        const isLocalhost =
            currentHost === "localhost" || currentHost === "127.0.0.1";
        const mainDomain = $appSettings.auth?.main_domain;
        const isMainDomain = mainDomain && currentHost === mainDomain;
        if (
            !isTauri &&
            !isLocalhost &&
            !isMainDomain &&
            customDomain &&
            currentHost !== customDomain
        ) {
            // Domain mismatch -> Logout and show error instead of redirecting
            const { logout } = await import("$lib/stores/auth");
            logout();
            error = "Invalid login credentials or unauthorized domain.";
            return;
        }

        // 3. Tenant User (Subdomain)
        if (slug) {
            // Check if we are already on the correct subdomain (slug.basedomain.com)
            // Or if we are using path-based routing (domain.com/slug/dashboard)

            // NOTE: This assumes standard "slug.basedomain.com" structure OR path-based "/slug/..."
            // If the current hostname DOES NOT contain the slug, and it's the main domain,
            // we should probably redirect to the subdomain if that's the architecture.
            // For now, let's stick to the existing path-based logic but make it robust.

            if ($page.url.hostname.includes(slug)) {
                // We are on the correct subdomain (presumably)
                if (u.role === "admin") {
                    goto(`/admin`); // Root of subdomain
                } else {
                    goto(`/dashboard`);
                }
            } else {
                // We are on main domain, redirect to path-based tenant dashboard
                if (u.role === "admin") {
                    goto(`/${slug}/admin`);
                } else {
                    goto(`/${slug}/dashboard`);
                }
            }
        } else {
            // Fallback for users without tenant (shouldn't happen usually)
            goto("/dashboard");
        }
    }

    async function handleSubmit(e: Event) {
        e.preventDefault();
        error = "";
        loading = true;

        try {
            const response = await login(email, password, rememberMe);

            if (response.requires_2fa) {
                tempToken = response.temp_token || "";
                available2FAMethods = response.available_2fa_methods || [
                    "totp",
                ];

                // If only one method, go directly to it
                if (available2FAMethods.length === 1) {
                    selected2FAMethod = available2FAMethods[0];
                    step =
                        available2FAMethods[0] === "email"
                            ? "2fa-email"
                            : "2fa-totp";

                    // Auto-send email OTP if email is the only method
                    if (available2FAMethods[0] === "email") {
                        await sendEmailOtp();
                    }
                } else {
                    step = "2fa-select";
                }
                return;
            }

            if (response.user) {
                // ... (existing domain check)
                const customDomain =
                    response.tenant?.custom_domain ||
                    response.user.tenant_custom_domain;
                const currentHost = window.location.hostname;

                if (
                    customDomain &&
                    currentHost !== customDomain &&
                    !response.user.is_super_admin
                ) {
                    error = "Invalid login credentials or unauthorized domain.";
                    // Clear session immediately
                    token.set(null);
                    user.set(null);
                    if (typeof window !== "undefined") {
                        localStorage.removeItem("auth_token");
                        sessionStorage.removeItem("auth_token");
                        localStorage.removeItem("auth_user");
                        sessionStorage.removeItem("auth_user");
                    }
                    loading = false;
                    return;
                }

                redirectUser(response.user, response.tenant);
            }
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
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

            if (response.token) {
                const { setAuthData } = await import("$lib/stores/auth");
                setAuthData(
                    response.token,
                    response.user,
                    rememberMe,
                    response.tenant,
                );

                redirectUser(response.user, response.tenant);
            }
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }
</script>

<div class="login-container">
    <div class="form-section">
        <div class="form-wrapper">
            <div class="form-header">
                <h2>
                    {#if step === "login"}
                        {$t("auth.login.title")}
                    {:else if step === "2fa-select"}
                        Two-Factor Authentication
                    {:else if step === "2fa-totp"}
                        Authenticator App
                    {:else if step === "2fa-email"}
                        Email Verification
                    {/if}
                </h2>
                <p>
                    {#if step === "login"}
                        {$t("auth.login.subtitle")}
                    {:else if step === "2fa-select"}
                        Choose your preferred verification method
                    {:else if step === "2fa-totp"}
                        Enter the 6-digit code from your authenticator app
                    {:else if step === "2fa-email"}
                        Enter the 6-digit code sent to your email
                    {/if}
                </p>
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
            {:else if step === "2fa-select"}
                <!-- 2FA Method Selection -->
                <div class="method-selection">
                    <p
                        style="margin-bottom: 1.5rem; color: var(--text-secondary);"
                    >
                        Choose your verification method:
                    </p>

                    {#each available2FAMethods as method}
                        <button
                            type="button"
                            class="method-btn"
                            on:click={() => selectMethod(method)}
                        >
                            <Icon
                                name={method === "totp" ? "shield" : "mail"}
                                size={24}
                            />
                            <span>
                                {method === "totp"
                                    ? "Authenticator App"
                                    : "Email Code"}
                            </span>
                        </button>
                    {/each}

                    <button
                        type="button"
                        class="btn-text"
                        on:click={() => (step = "login")}
                        style="width: 100%; margin-top: 1rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
                    >
                        Back to Login
                    </button>
                </div>
            {:else if step === "2fa-totp"}
                <!-- TOTP Form -->
                <form on:submit|preventDefault={handle2FAVerify}>
                    <div
                        class="input-group"
                        class:focus={activeField === "2fa"}
                    >
                        <label for="2fa-code">
                            {$t("auth.2fa.enter_code") ||
                                "Enter Verification Code"}
                        </label>
                        <div class="field">
                            <span class="icon"
                                ><Icon name="shield" size={18} /></span
                            >
                            <input
                                type="text"
                                id="2fa-code"
                                bind:value={twoFactorCode}
                                on:focus={() => (activeField = "2fa")}
                                on:blur={() => (activeField = "")}
                                placeholder={$t("common.otp_placeholder") ||
                                    "000000"}
                                maxlength="6"
                                required
                                style="letter-spacing: 0.5em; text-align: center;"
                            />
                        </div>
                    </div>

                    <div
                        class="form-utils"
                        style="margin-bottom: 1rem; justify-content: center;"
                    >
                        <label class="checkbox">
                            <input type="checkbox" bind:checked={trustDevice} />
                            <span class="checkmark"></span>
                            <span
                                >{$t("auth.2fa.trust_device") ||
                                    "Trust this device for 30 days"}</span
                            >
                        </label>
                    </div>

                    <button
                        type="submit"
                        class="btn-primary"
                        disabled={loading || twoFactorCode.length < 6}
                    >
                        {#if loading}
                            <div class="spinner"></div>
                        {:else}
                            {$t("auth.2fa.verify_and_login") ||
                                "Verify & Login"}
                        {/if}
                    </button>

                    {#if available2FAMethods.length > 1}
                        <button
                            type="button"
                            class="btn-text"
                            on:click={() => (step = "2fa-select")}
                            style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
                        >
                            {$t("auth.2fa.try_another_method") ||
                                "Try another method"}
                        </button>
                    {/if}

                    <button
                        type="button"
                        class="btn-text"
                        on:click={() => (step = "login")}
                        style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
                    >
                        {$t("auth.2fa.back_to_login") || "Back to Login"}
                    </button>
                </form>
            {:else if step === "2fa-email"}
                <!-- Email OTP Form -->
                <form on:submit|preventDefault={handle2FAVerify}>
                    {#if emailOtpSent}
                        <div
                            class="otp-sent-notice"
                            style="margin-bottom: 1rem; padding: 1rem; background: var(--bg-success); border-radius: 8px; color: var(--text-success);"
                        >
                            <Icon name="check-circle" size={18} />
                            <span>
                                {$t("auth.2fa.email_sent") ||
                                    "A verification code has been sent to your email"}
                            </span>
                        </div>
                    {/if}

                    <div
                        class="input-group"
                        class:focus={activeField === "2fa"}
                    >
                        <label for="email-otp-code">
                            {$t("auth.2fa.enter_email_code") ||
                                "Enter Email Code"}
                        </label>
                        <div class="field">
                            <span class="icon"
                                ><Icon name="mail" size={18} /></span
                            >
                            <input
                                type="text"
                                id="email-otp-code"
                                bind:value={twoFactorCode}
                                on:focus={() => (activeField = "2fa")}
                                on:blur={() => (activeField = "")}
                                placeholder={$t("common.otp_placeholder") ||
                                    "000000"}
                                maxlength="6"
                                required
                                style="letter-spacing: 0.5em; text-align: center;"
                            />
                        </div>
                    </div>

                    <div
                        class="form-utils"
                        style="margin-bottom: 1rem; justify-content: center;"
                    >
                        <label class="checkbox">
                            <input type="checkbox" bind:checked={trustDevice} />
                            <span class="checkmark"></span>
                            <span
                                >{$t("auth.2fa.trust_device") ||
                                    "Trust this device for 30 days"}</span
                            >
                        </label>
                    </div>

                    <button
                        type="submit"
                        class="btn-primary"
                        disabled={loading || twoFactorCode.length < 6}
                    >
                        {#if loading}
                            <div class="spinner"></div>
                        {:else}
                            {$t("auth.2fa.verify_and_login") ||
                                "Verify & Login"}
                        {/if}
                    </button>

                    <button
                        type="button"
                        class="btn-text"
                        on:click={() => sendEmailOtp(true)}
                        disabled={emailOtpSending || resendCountdown > 0}
                        style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--primary); cursor: pointer;"
                    >
                        {#if emailOtpSending}
                            Sending...
                        {:else if resendCountdown > 0}
                            {$t("auth.2fa.resend_code") || "Resend Code"} ({resendCountdown}s)
                        {:else}
                            {$t("auth.2fa.resend_code") || "Resend Code"}
                        {/if}
                    </button>

                    {#if available2FAMethods.length > 1}
                        <button
                            type="button"
                            class="btn-text"
                            on:click={() => (step = "2fa-select")}
                            style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
                        >
                            {$t("auth.2fa.try_another_method") ||
                                "Try another method"}
                        </button>
                    {/if}

                    <button
                        type="button"
                        class="btn-text"
                        on:click={() => (step = "login")}
                        style="width: 100%; margin-top: 0.5rem; background: none; border: none; color: var(--text-secondary); cursor: pointer;"
                    >
                        {$t("auth.2fa.back_to_login") || "Back to Login"}
                    </button>
                </form>
            {/if}

            {#if $allowRegistration}
                <p class="footer-text">
                    {$t("auth.login.footer_text")}
                    <a href="/register">{$t("auth.login.register_link")}</a>
                </p>
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

    .method-selection {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .method-btn {
        display: flex;
        align-items: center;
        gap: 1rem;
        width: 100%;
        padding: 1rem 1.5rem;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        color: var(--text-primary);
        cursor: pointer;
        transition: all 0.2s;
        font-size: 1rem;
    }

    .method-btn:hover {
        background: var(--bg-tertiary);
        border-color: var(--color-primary);
    }

    .otp-sent-notice {
        display: flex;
        align-items: center;
        gap: 0.5rem;
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

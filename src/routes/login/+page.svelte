<script lang="ts">
    import { login, isAuthenticated, isAdmin } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";
    import { get } from "svelte/store";
    import Icon from "$lib/components/Icon.svelte";

    let email = "";
    let password = "";
    let rememberMe = true;
    let error = "";
    let loading = false;
    let activeField = "";
    
    let showPassword = false;

    $: appName = $appSettings.app_name || "Platform Core";
    $: appDescription = $appSettings.app_description || "Enterprise-grade boilerplate built with Rust and SvelteKit. Secure, scalable, and lightweight.";

    onMount(async () => {
        await Promise.all([
            appSettings.init(),
            appLogo.init()
        ]);

        if ($isAuthenticated) {
            if (get(isAdmin)) {
                goto("/admin");
            } else {
                goto("/dashboard");
            }
        }
    });

    async function handleSubmit(e: Event) {
        e.preventDefault();
        error = "";
        loading = true;

        try {
            const response = await login(email, password, rememberMe);
            if (response.user.role === 'admin') {
                goto("/admin");
            } else {
                goto("/dashboard");
            }
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }
</script>

<div class="login-container">
    <div class="brand-section">
        <div class="brand-content" in:fade={{ duration: 1000 }}>
            <div class="logo-area">
                {#if $appLogo}
                    <img src={$appLogo} alt="App Logo" class="app-logo" />
                {:else}
                    <Icon name="app" size={48} strokeWidth={1.5} />
                {/if}
                <h1>{appName}</h1>
            </div>
            <p>{appDescription}</p>
        </div>
    </div>

    <div class="form-section">
        <div class="form-wrapper">
            <div class="form-header">
                <h2>Sign In</h2>
                <p>Access your dashboard</p>
            </div>

            {#if error}
                <div class="alert error" in:fly={{ y: -10 }}>
                    {error}
                </div>
            {/if}

            <form on:submit={handleSubmit}>
                <div class="input-group" class:focus={activeField === 'email'}>
                    <label for="email">Email</label>
                    <div class="field">
                        <span class="icon"><Icon name="mail" size={18} /></span>
                        <input
                            type="email"
                            id="email"
                            bind:value={email}
                            on:focus={() => activeField = 'email'}
                            on:blur={() => activeField = ''}
                            placeholder="name@company.com"
                            required
                        />
                    </div>
                </div>

                <div class="input-group" class:focus={activeField === 'password'}>
                    <label for="password">Password</label>
                    <div class="field">
                        <span class="icon"><Icon name="lock" size={18} /></span>
                        <input
                            type={showPassword ? "text" : "password"}
                            id="password"
                            bind:value={password}
                            on:focus={() => activeField = 'password'}
                            on:blur={() => activeField = ''}
                            placeholder="••••••••"
                            required
                            class="password-input"
                        />
                        <button 
                            type="button" 
                            class="toggle-password" 
                            on:click={() => showPassword = !showPassword}
                            tabindex="-1"
                        >
                            <Icon name={showPassword ? 'eye-off' : 'eye'} size={18} />
                        </button>
                    </div>
                </div>

                <div class="form-utils">
                    <label class="checkbox">
                        <input type="checkbox" bind:checked={rememberMe} />
                        <span class="checkmark"></span>
                        <span>Stay signed in</span>
                    </label>
                    <a href="/forgot-password">Reset password</a>
                </div>

                <button type="submit" class="btn-primary" disabled={loading}>
                    {#if loading}
                        <div class="spinner"></div>
                    {:else}
                        Continue
                    {/if}
                </button>
            </form>

            <p class="footer-text">
                Need an account? <a href="/register">Register here</a>
            </p>
        </div>
    </div>
</div>

<style>
    .login-container {
        display: grid;
        grid-template-columns: 1fr 1.2fr;
        min-height: 100vh;
        background: var(--bg-primary);
    }

    .brand-section {
        background: var(--bg-secondary);
        border-right: 1px solid var(--border-color);
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 4rem;
    }

    .brand-content {
        max-width: 400px;
    }

    .logo-area {
        color: var(--color-primary);
        margin-bottom: 2rem;
        display: flex;
        flex-direction: column;
        align-items: center; /* Center content */
    }

    .app-logo {
        max-width: 120px;
        max-height: 120px;
        margin-bottom: 1rem;
        object-fit: contain;
    }

    .logo-area h1 {
        margin-top: 1.5rem;
        font-size: 2.5rem;
        font-weight: 800;
        color: var(--text-primary);
        letter-spacing: -1px;
    }

    .brand-content p {
        color: var(--text-secondary);
        font-size: 1.1rem;
        line-height: 1.6;
    }

    .form-section {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2rem;
    }

    .form-wrapper {
        width: 100%;
        max-width: 360px;
    }

    .form-header {
        margin-bottom: 2.5rem;
    }

    .form-header h2 {
        font-size: 1.75rem;
        font-weight: 700;
        color: var(--text-primary);
    }

    .form-header p {
        color: var(--text-secondary);
        margin-top: 0.25rem;
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

    .checkbox input { display: none; }

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
        left: 5px; top: 2px;
        width: 3px; height: 7px;
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

    .btn-primary:hover { opacity: 0.9; }
    .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

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
        border: 2px solid rgba(255,255,255,0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }

    @keyframes spin { to { transform: rotate(360deg); } }

    @media (max-width: 800px) {
        .login-container { grid-template-columns: 1fr; }
        .brand-section { display: none; }
    }
</style>
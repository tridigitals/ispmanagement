<script lang="ts">
    import {
        register as registerUser,
        isAuthenticated,
    } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import Icon from "$lib/components/Icon.svelte";

    let name = "";
    let email = "";
    let password = "";
    let confirmPassword = "";
    let error = "";
    let loading = false;
    
    // Visibility states
    let showPassword = false;
    let showConfirmPassword = false;

    // Default policy if store not loaded yet
    $: policy = $appSettings.auth || {
        password_min_length: 8,
        password_require_uppercase: true,
        password_require_number: true,
        password_require_special: false
    };

    onMount(async () => {
        if ($isAuthenticated) {
            goto("/dashboard");
        }
        await appSettings.init();
    });

    function validatePassword(pwd: string): string | null {
        if (pwd.length < policy.password_min_length) {
            return `Password must be at least ${policy.password_min_length} characters`;
        }
        if (policy.password_require_uppercase && !/[A-Z]/.test(pwd)) {
            return "Password must contain at least one uppercase letter";
        }
        if (policy.password_require_number && !/[0-9]/.test(pwd)) {
            return "Password must contain at least one number";
        }
        if (policy.password_require_special && !/[!@#$%^&*()_+\-=[\]{}|;:',.<>?/`~]/.test(pwd)) {
            return "Password must contain at least one special character";
        }
        return null;
    }

    async function handleSubmit(e: Event) {
        e.preventDefault();
        error = "";

        // Validate passwords match
        if (password !== confirmPassword) {
            error = "Passwords do not match";
            return;
        }

        // Validate password against policy
        const policyError = validatePassword(password);
        if (policyError) {
            error = policyError;
            return;
        }

        loading = true;

        try {
            await registerUser(email, password, name);
            goto("/dashboard");
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }
</script>

<div class="auth-page">
    <div class="auth-card">
        <div class="auth-header">
            <h1>Create Account</h1>
            <p>Join us today</p>
        </div>

        {#if error}
            <div class="alert alert-error">
                {error}
            </div>
        {/if}

        <form on:submit={handleSubmit}>
            <div class="form-group">
                <label class="form-label" for="name">Full Name</label>
                <input
                    type="text"
                    id="name"
                    class="form-input"
                    bind:value={name}
                    placeholder="John Doe"
                    required
                    disabled={loading}
                />
            </div>

            <div class="form-group">
                <label class="form-label" for="email">Email</label>
                <input
                    type="email"
                    id="email"
                    class="form-input"
                    bind:value={email}
                    placeholder="you@example.com"
                    required
                    disabled={loading}
                />
            </div>

            <div class="form-group">
                <label class="form-label" for="password">Password</label>
                <div class="input-wrapper">
                    <input
                        type={showPassword ? "text" : "password"}
                        id="password"
                        class="form-input"
                        bind:value={password}
                        placeholder="••••••••"
                        required
                        minlength={policy.password_min_length}
                        disabled={loading}
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
                <div class="password-hint">
                    <small>
                        Min {policy.password_min_length} chars
                        {#if policy.password_require_uppercase}, 1 uppercase{/if}
                        {#if policy.password_require_number}, 1 number{/if}
                        {#if policy.password_require_special}, 1 special char{/if}
                    </small>
                </div>
            </div>

            <div class="form-group">
                <label class="form-label" for="confirmPassword"
                    >Confirm Password</label
                >
                <div class="input-wrapper">
                    <input
                        type={showConfirmPassword ? "text" : "password"}
                        id="confirmPassword"
                        class="form-input"
                        bind:value={confirmPassword}
                        placeholder="••••••••"
                        required
                        disabled={loading}
                    />
                    <button 
                        type="button" 
                        class="toggle-password" 
                        on:click={() => showConfirmPassword = !showConfirmPassword}
                        tabindex="-1"
                    >
                        <Icon name={showConfirmPassword ? 'eye-off' : 'eye'} size={18} />
                    </button>
                </div>
            </div>

            <button
                type="submit"
                class="btn btn-primary w-full"
                disabled={loading}
            >
                {#if loading}
                    Creating Account...
                {:else}
                    Create Account
                {/if}
            </button>
        </form>

        <div class="auth-footer">
            <p>
                Already have an account?
                <a href="/login">Sign in</a>
            </p>
        </div>
    </div>
</div>

<style>
    .auth-page {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2rem;
    }

    .auth-card {
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        padding: 2.5rem;
        width: 100%;
        max-width: 420px;
        backdrop-filter: blur(10px);
    }

    .auth-header {
        text-align: center;
        margin-bottom: 2rem;
    }

    .auth-header h1 {
        font-size: 1.75rem;
        margin-bottom: 0.5rem;
    }

    .auth-header p {
        color: var(--text-secondary);
    }

    .auth-footer {
        text-align: center;
        margin-top: 1.5rem;
        padding-top: 1.5rem;
        border-top: 1px solid var(--border-color);
    }

    .auth-footer p {
        color: var(--text-secondary);
    }
    
    .password-hint {
        margin-top: 0.5rem;
        color: var(--text-secondary);
        font-size: 0.8rem;
        line-height: 1.4;
    }

    /* Input Wrapper for Toggle Icon */
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
    
    /* Ensure input has space for icon */
    .input-wrapper .form-input {
        padding-right: 40px;
    }
</style>

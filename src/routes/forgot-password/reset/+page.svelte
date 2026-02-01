<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/stores';
    import { auth } from '$lib/api/client';
    import { goto } from '$app/navigation';
    import Icon from '$lib/components/Icon.svelte';
    import { t } from "svelte-i18n";
    import { get } from "svelte/store";

    let token = '';
    let password = '';
    let confirmPassword = '';
    let error = '';
    let success = false;
    let loading = false;
    let showPassword = false;
    let showConfirmPassword = false;

    onMount(() => {
        token = $page.url.searchParams.get('token') || '';
        if (!token) {
            error =
                get(t)("auth.reset_password.invalid_token") ||
                'Invalid or missing reset token.';
        }
    });

    async function handleSubmit() {
        if (!token) return;
        if (password !== confirmPassword) {
            error =
                get(t)("auth.reset_password.passwords_do_not_match") ||
                'Passwords do not match';
            return;
        }
        if (password.length < 8) {
            error =
                get(t)("auth.reset_password.min_length") ||
                'Password must be at least 8 characters';
            return;
        }

        loading = true;
        error = '';

        try {
            await auth.resetPassword(token, password);
            success = true;
            setTimeout(() => {
                goto('/login');
            }, 3000);
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
            <h1>{$t("auth.reset_password.title") || "Reset Password"}</h1>
            <p>
                {$t("auth.reset_password.subtitle") ||
                    "Enter your new password below"}
            </p>
        </div>

        {#if error}
            <div class="alert alert-error">
                {error}
            </div>
        {/if}

        {#if success}
            <div class="alert alert-success">
                {$t("auth.reset_password.success") ||
                    "Password reset successfully!"}
                {$t("auth.reset_password.redirecting") ||
                    "Redirecting to login..."}
            </div>
        {:else}
            <form on:submit|preventDefault={handleSubmit}>
                <div class="form-group">
                    <label class="form-label" for="password"
                        >{$t("auth.reset_password.new_password") ||
                            "New Password"}</label
                    >
                    <div class="input-wrapper">
                        <input
                            type={showPassword ? "text" : "password"}
                            id="password"
                            class="form-input"
                            bind:value={password}
                            placeholder="••••••••"
                            required
                            minlength="8"
                            disabled={loading}
                        />
                        <button 
                            type="button" 
                            class="toggle-password" 
                            on:click={() => showPassword = !showPassword}
                        >
                            <Icon name={showPassword ? 'eye-off' : 'eye'} size={18} />
                        </button>
                    </div>
                </div>

                <div class="form-group">
                    <label class="form-label" for="confirmPassword"
                        >{$t("auth.reset_password.confirm_password") ||
                            "Confirm Password"}</label
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
                        >
                            <Icon name={showConfirmPassword ? 'eye-off' : 'eye'} size={18} />
                        </button>
                    </div>
                </div>

                <button
                    type="submit"
                    class="btn btn-primary w-full"
                    disabled={loading || !token}
                >
                    {#if loading}
                        {$t("auth.reset_password.resetting") || "Resetting..."}
                    {:else}
                        {$t("auth.reset_password.submit") || "Reset Password"}
                    {/if}
                </button>
            </form>
        {/if}
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
        border-radius: var(--radius-lg);
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

    .form-group {
        margin-bottom: 1.5rem;
    }

    .form-label {
        display: block;
        margin-bottom: 0.5rem;
        font-weight: 500;
        font-size: 0.95rem;
    }

    .form-input {
        width: 100%;
        padding: 0.75rem 1rem;
        border-radius: var(--radius-md);
        border: 1px solid var(--border-color);
        background: var(--bg-surface);
        color: var(--text-primary);
        font-size: 1rem;
        transition: all 0.2s;
    }

    .form-input:focus {
        outline: none;
        border-color: var(--color-primary);
        box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
    }

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
    }

    .btn {
        width: 100%;
        padding: 0.75rem;
        border-radius: var(--radius-md);
        font-weight: 600;
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

    .btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .alert {
        padding: 1rem;
        margin-bottom: 1.5rem;
        border-radius: var(--radius-md);
        font-weight: 500;
    }

    .alert-error {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.2);
        color: #ef4444;
    }

    .alert-success {
        background: rgba(34, 197, 94, 0.1);
        border: 1px solid rgba(34, 197, 94, 0.2);
        color: #22c55e;
    }
</style>

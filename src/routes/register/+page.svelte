<script lang="ts">
    import {
        register as registerUser,
        isAuthenticated,
    } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";

    let name = "";
    let email = "";
    let password = "";
    let confirmPassword = "";
    let error = "";
    let loading = false;

    onMount(() => {
        if ($isAuthenticated) {
            goto("/dashboard");
        }
    });

    async function handleSubmit(e: Event) {
        e.preventDefault();
        error = "";

        // Validate passwords match
        if (password !== confirmPassword) {
            error = "Passwords do not match";
            return;
        }

        // Validate password length
        if (password.length < 8) {
            error = "Password must be at least 8 characters";
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
                <input
                    type="password"
                    id="password"
                    class="form-input"
                    bind:value={password}
                    placeholder="••••••••"
                    required
                    minlength="8"
                    disabled={loading}
                />
            </div>

            <div class="form-group">
                <label class="form-label" for="confirmPassword"
                    >Confirm Password</label
                >
                <input
                    type="password"
                    id="confirmPassword"
                    class="form-input"
                    bind:value={confirmPassword}
                    placeholder="••••••••"
                    required
                    disabled={loading}
                />
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
</style>

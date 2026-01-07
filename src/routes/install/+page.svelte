<script lang="ts">
    import { install } from "$lib/api/client";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";

    let name = "";
    let email = "";
    let password = "";
    let confirmPassword = "";
    let error = "";
    let loading = false;
    let step = 1; // 1: Welcome, 2: Account Setup, 3: Success

    onMount(async () => {
        // Double check not already installed
        try {
            const isInstalled = await install.checkIsInstalled();
            if (isInstalled) {
                goto("/login");
            }
        } catch (e) {
            console.error(e);
        }
    });

    async function handleSubmit() {
        error = "";
        if (!name || !email || !password || !confirmPassword) {
            error = "Please fill in all fields";
            return;
        }

        if (password !== confirmPassword) {
            error = "Passwords do not match";
            return;
        }

        if (password.length < 8) {
            error = "Password must be at least 8 characters";
            return;
        }

        loading = true;
        try {
            await install.installApp(name, email, password);
            step = 3;
            // Delay redirect slightly to show success
            setTimeout(() => {
                goto("/login");
            }, 2000);
        } catch (e: any) {
            error = e.message || "Installation failed";
        } finally {
            loading = false;
        }
    }
</script>

<div class="install-container">
    <div class="card">
        {#if step === 1}
            <div class="step-content">
                <div class="icon-wrapper">
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="48"
                        height="48"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        ><path d="M12 2L2 7l10 5 10-5-10-5z" /><path
                            d="M2 17l10 5 10-5"
                        /><path d="M2 12l10 5 10-5" /></svg
                    >
                </div>
                <h1>Welcome to SaaS App</h1>
                <p>
                    Let's get your application set up. We'll start by creating
                    your administrator account.
                </p>
                <button class="btn-primary" on:click={() => (step = 2)}>
                    Get Started
                </button>
            </div>
        {:else if step === 2}
            <div class="step-content">
                <h2>Create Admin Account</h2>
                <p class="subtitle">
                    This account will have full access to the system.
                </p>

                {#if error}
                    <div class="error-alert">
                        {error}
                    </div>
                {/if}

                <form on:submit|preventDefault={handleSubmit}>
                    <div class="form-group">
                        <label for="name">Full Name</label>
                        <input
                            type="text"
                            id="name"
                            bind:value={name}
                            placeholder="John Doe"
                            disabled={loading}
                        />
                    </div>

                    <div class="form-group">
                        <label for="email">Email Address</label>
                        <input
                            type="email"
                            id="email"
                            bind:value={email}
                            placeholder="john@example.com"
                            disabled={loading}
                        />
                    </div>

                    <div class="form-group">
                        <label for="password">Password</label>
                        <input
                            type="password"
                            id="password"
                            bind:value={password}
                            placeholder="••••••••"
                            disabled={loading}
                        />
                    </div>

                    <div class="form-group">
                        <label for="confirmPassword">Confirm Password</label>
                        <input
                            type="password"
                            id="confirmPassword"
                            bind:value={confirmPassword}
                            placeholder="••••••••"
                            disabled={loading}
                        />
                    </div>

                    <div class="actions">
                        <button
                            type="button"
                            class="btn-secondary"
                            on:click={() => (step = 1)}
                            disabled={loading}>Back</button
                        >
                        <button
                            type="submit"
                            class="btn-primary"
                            disabled={loading}
                        >
                            {#if loading}
                                Installing...
                            {:else}
                                Complete Setup
                            {/if}
                        </button>
                    </div>
                </form>
            </div>
        {:else if step === 3}
            <div class="step-content success">
                <div class="success-icon">
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="48"
                        height="48"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        ><path
                            d="M22 11.08V12a10 10 0 1 1-5.93-9.14"
                        /><polyline points="22 4 12 14.01 9 11.01" /></svg
                    >
                </div>
                <h2>Installation Complete!</h2>
                <p>Redirecting you to login...</p>
            </div>
        {/if}
    </div>
</div>

<style>
    .install-container {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--bg-primary);
        padding: 1rem;
    }

    .card {
        background: var(--bg-secondary);
        border: 1px solid var(--border-primary);
        border-radius: 1rem;
        padding: 2.5rem;
        width: 100%;
        max-width: 480px;
        box-shadow: var(--shadow-lg);
    }

    .step-content {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        text-align: center;
    }

    .icon-wrapper {
        width: 80px;
        height: 80px;
        background: var(--bg-tertiary);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0 auto;
        color: var(--color-primary);
    }

    h1,
    h2 {
        color: var(--text-primary);
        margin: 0;
    }

    h1 {
        font-size: 1.75rem;
    }
    h2 {
        font-size: 1.5rem;
    }

    p {
        color: var(--text-secondary);
        margin: 0;
        line-height: 1.6;
    }

    .subtitle {
        font-size: 0.95rem;
        margin-bottom: 0.5rem;
    }

    form {
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
        text-align: left;
    }

    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    label {
        font-size: 0.9rem;
        font-weight: 500;
        color: var(--text-primary);
    }

    input {
        padding: 0.75rem;
        border-radius: 0.5rem;
        border: 1px solid var(--border-primary);
        background: var(--bg-tertiary);
        color: var(--text-primary);
        font-size: 1rem;
        transition: all 0.2s;
    }

    input:focus {
        border-color: var(--color-primary);
        outline: none;
        box-shadow: 0 0 0 2px var(--color-primary-transparent);
    }

    .actions {
        display: flex;
        gap: 1rem;
        margin-top: 0.5rem;
    }

    button {
        flex: 1;
        padding: 0.75rem;
        border-radius: 0.5rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
    }

    .btn-primary {
        background: var(--color-primary);
        color: white;
    }

    .btn-primary:hover:not(:disabled) {
        opacity: 0.9;
    }

    .btn-secondary {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .btn-secondary:hover:not(:disabled) {
        background: var(--border-primary);
    }

    button:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    .error-alert {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.2);
        color: #ef4444;
        padding: 0.75rem;
        border-radius: 0.5rem;
        font-size: 0.9rem;
    }

    .success {
        padding: 2rem 0;
    }

    .success-icon {
        color: #10b981;
        margin-bottom: 1rem;
    }
</style>

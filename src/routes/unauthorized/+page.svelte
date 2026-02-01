<script lang="ts">
    import { goto } from '$app/navigation';
    import { user } from '$lib/stores/auth';
    import { t } from "svelte-i18n";
</script>

<div class="unauthorized-page fade-in">
    <div class="content">
        <div class="lock-icon">🛡️</div>
        <div class="error-code">403</div>
        <h1>{$t("pages.unauthorized.title") || "Access Denied"}</h1>
        <p class="message">
            {$t("pages.unauthorized.sorry") || "Sorry,"}
            <strong>{$user?.name || $t("profile.fallback.user") || "User"}</strong>.
            {$t("pages.unauthorized.message") ||
                "You don't have the required permissions to view this page. This area is restricted to system administrators."}
        </p>
        
        <div class="actions">
            <button class="btn btn-primary" on:click={() => goto('/dashboard')}>
                {$t("pages.unauthorized.back") || "Return to Safety"}
            </button>
        </div>
    </div>
</div>

<style>
    .unauthorized-page {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2rem;
        text-align: center;
        background: var(--bg-primary);
    }

    .content {
        max-width: 500px;
        padding: 3rem;
        background: var(--bg-card);
        border: 1px solid var(--border-color);
        border-radius: var(--border-radius);
        backdrop-filter: blur(10px);
    }

    .lock-icon {
        font-size: 4rem;
        margin-bottom: 1rem;
    }

    .error-code {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--color-danger);
        margin-bottom: 0.5rem;
        letter-spacing: 2px;
    }

    h1 {
        font-size: 2rem;
        margin-bottom: 1rem;
        color: var(--text-primary);
    }

    .message {
        font-size: 1rem;
        color: var(--text-secondary);
        margin-bottom: 2rem;
        line-height: 1.6;
    }

    .message strong {
        color: var(--text-primary);
    }

    .actions {
        display: flex;
        gap: 1rem;
        justify-content: center;
    }

    @keyframes fadeIn {
        from { opacity: 0; transform: scale(0.95); }
        to { opacity: 1; transform: scale(1); }
    }

    .fade-in {
        animation: fadeIn 0.4s ease-out;
    }
</style>

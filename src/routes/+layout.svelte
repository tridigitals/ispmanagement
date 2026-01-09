<script lang="ts">
    import "$lib/styles/global.css";
    import "../lib/i18n"; // Init i18n
    import { waitLocale } from "svelte-i18n";
    import { checkAuth } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { theme } from "$lib/stores/theme";
    import { install } from "$lib/api/client";
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";

    let loading = true;

    onMount(async () => {
        try {
            // Apply saved theme
            theme.init();
            
            // Load global settings & logo immediately
            await Promise.all([
                appSettings.init(),
                appLogo.init()
            ]);
            
            // Wait for i18n to be ready (locale set in appSettings.init)
            await waitLocale();

            // Check if app is installed
            const isInstalled = await install.checkIsInstalled();
            const currentPath = $page.url.pathname;

            if (!isInstalled) {
                if (currentPath !== "/install") {
                    goto("/install");
                }
            } else {
                if (currentPath === "/install") {
                    goto("/login");
                }
                await checkAuth();
            }
        } catch (e) {
            console.error("Failed to check installation status:", e);
        } finally {
            loading = false;
        }
    });
</script>

{#if loading}
    <div class="loading-container">
        <div class="spinner"></div>
        <p>Loading...</p>
    </div>
{:else}
    <slot />
{/if}

<style>
    .loading-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
        gap: 1rem;
    }

    .spinner {
        width: 40px;
        height: 40px;
        border: 3px solid var(--bg-tertiary);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
</style>

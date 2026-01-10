<script lang="ts">
    import "$lib/styles/global.css";
    import "../lib/i18n"; // Init i18n
    import { waitLocale } from "svelte-i18n";
    import { checkAuth, isAuthenticated } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { appLogo } from "$lib/stores/logo";
    import { theme } from "$lib/stores/theme";
    import { install } from "$lib/api/client";
    import { onMount, onDestroy } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";
    import {
        connectWebSocket,
        disconnectWebSocket,
    } from "$lib/stores/websocket";

    let loading = true;

    onMount(async () => {
        try {
            // 1. Validate Auth & Session first
            // This ensures we have the correct tenant context before fetching data
            await checkAuth();

            // Apply saved theme
            theme.init();

            // Load global settings & logo from cache immediately
            // Logo is only refreshed from backend when user logs in (handled by auth store)
            await Promise.all([appSettings.init(), appLogo.init()]);

            // Wait for i18n to be ready (locale set in appSettings.init)
            await waitLocale();

            // Check if app is installed
            const isInstalled = await install.checkIsInstalled();
            const currentPath = $page.url.pathname;

            if (!isInstalled) {
                if (currentPath !== "/install") {
                    // console.log("App not installed, redirecting to /install");
                    goto("/install");
                }
            } else {
                if (currentPath === "/install") {
                    console.log(
                        "App installed, leaving /install page for /login",
                    );
                    goto("/login");
                }
                await checkAuth();

                // Connect to WebSocket for real-time updates (only if authenticated)
                if ($isAuthenticated) {
                    connectWebSocket();
                }
            }
        } catch (e) {
            console.error(
                "Critical Error during app initialization in +layout.svelte:",
                e,
            );
            // We stop here to prevent loops. The user will see a loading screen,
            // and the real error will be in the console.
        } finally {
            loading = false;
        }
    });

    // Disconnect WebSocket when app unloads
    onDestroy(() => {
        disconnectWebSocket();
    });
</script>

<svelte:head>
    {#if $appLogo}
        <link rel="icon" type="image/png" href={$appLogo} />
    {/if}
</svelte:head>

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

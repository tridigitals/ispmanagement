<script lang="ts">
    import Sidebar from "$lib/components/Sidebar.svelte";
    import Topbar from "$lib/components/Topbar.svelte";
    import { isAuthenticated, isSuperAdmin } from "$lib/stores/auth";
    import { appSettings } from "$lib/stores/settings";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";

    import { page } from "$app/stores";
    import { user } from "$lib/stores/auth";

    let { children } = $props();

    // Reactive Auth Guard & Tenant Scoping
    $effect(() => {
        if (!$isAuthenticated) {
            goto("/");
        } else {
            // Check maintenance mode - redirect non-superadmin users
            const settings = $appSettings as any;
            const isMaintenanceMode =
                settings.maintenance_mode === true ||
                settings.maintenance_mode === "true";

            if (isMaintenanceMode && !$isSuperAdmin) {
                goto("/maintenance");
                return;
            }

            // Check if current URL slug matches user's assigned tenant slug
            const currentSlug = $page.params.tenant;
            const userSlug = $user?.tenant_slug;
            
            // Custom Domain Enforcement
            const userCustomDomain = $user?.tenant_custom_domain;
            const currentHost = $page.url.hostname;

            if (userCustomDomain && currentHost !== userCustomDomain && !$isSuperAdmin) {
                console.warn(`[Layout] Domain Mismatch! Redirecting to ${userCustomDomain}`);
                window.location.href = `${window.location.protocol}//${userCustomDomain}/dashboard`;
                return;
            }

            if (
                currentSlug &&
                userSlug &&
                currentSlug.toLowerCase() !== userSlug.toLowerCase()
            ) {
                console.warn(
                    `[Layout] Tenant Mismatch! User ${userSlug} tried to access ${currentSlug}`,
                );
                // Strict Isolation: Logout user if they try to access a different tenant's area
                import("$lib/stores/auth").then((m) => m.logout());
                goto(`/${currentSlug}`);
            }
        }
    });

    onMount(() => {
        if (!$isAuthenticated) {
            goto("/");
        }

        // Check maintenance mode on mount
        const settings = $appSettings as any;
        const isMaintenanceMode =
            settings.maintenance_mode === true ||
            settings.maintenance_mode === "true";

        if (isMaintenanceMode && !$isSuperAdmin) {
            goto("/maintenance");
        }
    });

    let mobileOpen = $state(false);
</script>

<div class="app-shell">
    <!-- Sidebar sits on the base layer -->
    <Sidebar bind:isMobileOpen={mobileOpen} />

    <!-- Main Area is a floating card -->
    <div class="main-viewport">
        <div class="content-surface">
            <Topbar onMobileMenuClick={() => (mobileOpen = !mobileOpen)} />
            <div class="scroll-area">
                {@render children()}
            </div>
        </div>
    </div>
</div>

<style>
    .app-shell {
        display: flex;
        height: 100vh;
        height: 100dvh; /* Fallback & Modern unit */
        width: 100vw;
        background: var(--bg-app); /* Background dasar aplikasi */
        overflow: hidden;
    }

    .main-viewport {
        flex: 1;
        display: flex;
        flex-direction: column;
        padding: 8px 8px 8px 0; /* Padding untuk efek floating, kecuali kiri (nempel sidebar) */
    }

    .content-surface {
        flex: 1;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: var(--radius-lg); /* Sudut membulat modern */
        display: flex;
        flex-direction: column;
        overflow: hidden;
        box-shadow: var(--shadow-sm);
        position: relative;
    }

    .scroll-area {
        flex: 1;
        overflow-y: auto;
        position: relative;
    }
</style>

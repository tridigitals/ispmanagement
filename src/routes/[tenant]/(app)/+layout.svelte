<script>
    import Sidebar from "$lib/components/Sidebar.svelte";
    import Topbar from "$lib/components/Topbar.svelte";
    import { isAuthenticated } from "$lib/stores/auth";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import Toast from "$lib/components/Toast.svelte";

    import { page } from "$app/stores";
    import { user } from "$lib/stores/auth";

    // Reactive Auth Guard & Tenant Scoping
    $: {
        if (!$isAuthenticated) {
            goto("/");
        } else {
            // Check if current URL slug matches user's assigned tenant slug
            const currentSlug = $page.params.tenant;
            const userSlug = $user?.tenant_slug;

            console.log("[Layout] Debug:", {
                currentSlug,
                userSlug,
                user: $user,
            });

            if (currentSlug && userSlug && currentSlug !== userSlug) {
                console.warn(
                    `[Layout] Mismatch! Redirecting from ${currentSlug} to ${userSlug}`,
                );
                // Redirect to their correct dashboard
                goto(`/${userSlug}/dashboard`);
            }
        }
    }

    onMount(() => {
        if (!$isAuthenticated) {
            goto("/");
        }
    });

    let mobileOpen = false;
</script>

<div class="app-shell">
    <Toast />
    <!-- Sidebar sits on the base layer -->
    <Sidebar bind:isMobileOpen={mobileOpen} />

    <!-- Main Area is a floating card -->
    <div class="main-viewport">
        <div class="content-surface">
            <Topbar onMobileMenuClick={() => (mobileOpen = !mobileOpen)} />
            <div class="scroll-area">
                <slot />
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

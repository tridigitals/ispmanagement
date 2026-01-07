<script>
    import Sidebar from '$lib/components/Sidebar.svelte';
    import Topbar from '$lib/components/Topbar.svelte';
    import { isAuthenticated } from '$lib/stores/auth';
    import { goto } from '$app/navigation';
    import { onMount } from 'svelte';

    // Reactive Auth Guard
    $: if (!$isAuthenticated) {
        goto('/login');
    }

    onMount(() => {
        if (!$isAuthenticated) {
            goto('/login');
        }
    });
</script>

<div class="app-layout">
    <Sidebar />
    <main class="main-area">
        <Topbar />
        <div class="content-wrapper">
            <slot />
        </div>
    </main>
</div>

<style>
    .app-layout {
        display: flex;
        height: 100vh;
        overflow: hidden;
        background: var(--bg-primary);
    }

    .main-area {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .content-wrapper {
        flex: 1;
        overflow-y: auto;
        position: relative;
    }
</style>
<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { appSettings } from "$lib/stores/settings";
    import { isSuperAdmin } from "$lib/stores/auth";
    import { goto } from "$app/navigation";

    let message =
        "We're updating our systems to serve you better. Please check back soon!";
    let dots = "";
    let interval: any;

    onMount(async () => {
        // Get maintenance settings
        const settings = $appSettings as any;
        const isMaintenanceEnabled =
            settings.maintenance_mode === true ||
            settings.maintenance_mode === "true";

        // If maintenance mode is not enabled, redirect away from this page
        if (!isMaintenanceEnabled) {
            goto("/login");
            return;
        }

        // If superadmin, redirect to dashboard
        if ($isSuperAdmin) {
            goto("/dashboard");
            return;
        }

        // Get maintenance message from settings
        if (settings.maintenance_message) {
            message = settings.maintenance_message;
        }

        // Animated dots
        interval = setInterval(() => {
            dots = dots.length >= 3 ? "" : dots + ".";
        }, 500);
    });

    onDestroy(() => {
        if (interval) clearInterval(interval);
    });
</script>

<div class="maintenance-container">
    <div class="background-shapes">
        <div class="shape shape-1"></div>
        <div class="shape shape-2"></div>
        <div class="shape shape-3"></div>
    </div>

    <div class="maintenance-card">
        <div class="gear-container">
            <div class="gear gear-large">
                <svg viewBox="0 0 24 24" fill="currentColor">
                    <path
                        d="M12 15.5A3.5 3.5 0 0 1 8.5 12A3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5a3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97c0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1c0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66Z"
                    />
                </svg>
            </div>
            <div class="gear gear-small">
                <svg viewBox="0 0 24 24" fill="currentColor">
                    <path
                        d="M12 15.5A3.5 3.5 0 0 1 8.5 12A3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5a3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97c0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1c0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66Z"
                    />
                </svg>
            </div>
        </div>

        <h1>Under Maintenance</h1>
        <p class="message">{message}</p>

        <div class="progress-container">
            <div class="progress-bar">
                <div class="progress-fill"></div>
            </div>
            <span class="progress-text">Working on it{dots}</span>
        </div>

        <p class="footer-text">
            Thank you for your patience. We'll be back shortly!
        </p>
    </div>
</div>

<style>
    .maintenance-container {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: linear-gradient(
            135deg,
            #0f0f23 0%,
            #1a1a3e 50%,
            #0f0f23 100%
        );
        padding: 1rem;
        position: relative;
        overflow: hidden;
    }

    .background-shapes {
        position: absolute;
        inset: 0;
        overflow: hidden;
        pointer-events: none;
    }

    .shape {
        position: absolute;
        border-radius: 50%;
        filter: blur(80px);
        opacity: 0.3;
    }

    .shape-1 {
        width: 400px;
        height: 400px;
        background: linear-gradient(135deg, #667eea, #764ba2);
        top: -100px;
        left: -100px;
        animation: float 8s ease-in-out infinite;
    }

    .shape-2 {
        width: 300px;
        height: 300px;
        background: linear-gradient(135deg, #f093fb, #f5576c);
        bottom: -50px;
        right: -50px;
        animation: float 10s ease-in-out infinite reverse;
    }

    .shape-3 {
        width: 200px;
        height: 200px;
        background: linear-gradient(135deg, #4facfe, #00f2fe);
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        animation: pulse 4s ease-in-out infinite;
    }

    @keyframes float {
        0%,
        100% {
            transform: translateY(0) rotate(0deg);
        }
        50% {
            transform: translateY(-30px) rotate(10deg);
        }
    }

    @keyframes pulse {
        0%,
        100% {
            opacity: 0.2;
            transform: translate(-50%, -50%) scale(1);
        }
        50% {
            opacity: 0.4;
            transform: translate(-50%, -50%) scale(1.1);
        }
    }

    .maintenance-card {
        background: rgba(255, 255, 255, 0.05);
        backdrop-filter: blur(20px);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 24px;
        padding: 3rem;
        text-align: center;
        max-width: 520px;
        width: 100%;
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
        position: relative;
        z-index: 1;
    }

    .gear-container {
        position: relative;
        width: 120px;
        height: 120px;
        margin: 0 auto 2rem;
    }

    .gear {
        position: absolute;
        color: rgba(255, 255, 255, 0.8);
    }

    .gear-large {
        width: 80px;
        height: 80px;
        top: 0;
        left: 0;
        animation: rotate 4s linear infinite;
    }

    .gear-small {
        width: 50px;
        height: 50px;
        bottom: 10px;
        right: 0;
        animation: rotate 3s linear infinite reverse;
    }

    @keyframes rotate {
        from {
            transform: rotate(0deg);
        }
        to {
            transform: rotate(360deg);
        }
    }

    h1 {
        font-size: 2rem;
        font-weight: 700;
        margin: 0 0 1rem 0;
        color: #fff;
        text-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
    }

    .message {
        color: rgba(255, 255, 255, 0.7);
        font-size: 1.1rem;
        line-height: 1.6;
        margin: 0 0 2rem 0;
    }

    .progress-container {
        margin-bottom: 2rem;
    }

    .progress-bar {
        width: 100%;
        height: 6px;
        background: rgba(255, 255, 255, 0.1);
        border-radius: 3px;
        overflow: hidden;
        margin-bottom: 0.75rem;
    }

    .progress-fill {
        height: 100%;
        background: linear-gradient(90deg, #667eea, #764ba2, #f093fb);
        background-size: 200% 100%;
        border-radius: 3px;
        animation: shimmer 2s ease-in-out infinite;
    }

    @keyframes shimmer {
        0% {
            background-position: 100% 0;
            width: 20%;
        }
        50% {
            background-position: 0 0;
            width: 80%;
        }
        100% {
            background-position: 100% 0;
            width: 20%;
        }
    }

    .progress-text {
        color: rgba(255, 255, 255, 0.5);
        font-size: 0.9rem;
        font-family: monospace;
    }

    .features {
        display: flex;
        justify-content: center;
        gap: 1.5rem;
        flex-wrap: wrap;
        margin-bottom: 2rem;
    }

    .feature {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.5rem;
        padding: 1rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 12px;
        min-width: 120px;
    }

    .feature-icon {
        font-size: 1.5rem;
    }

    .feature span {
        color: rgba(255, 255, 255, 0.6);
        font-size: 0.8rem;
        text-align: center;
    }

    .footer-text {
        color: rgba(255, 255, 255, 0.4);
        font-size: 0.85rem;
        margin: 0;
    }

    @media (max-width: 480px) {
        .maintenance-card {
            padding: 2rem 1.5rem;
        }

        h1 {
            font-size: 1.5rem;
        }

        .features {
            gap: 0.75rem;
        }

        .feature {
            min-width: 90px;
            padding: 0.75rem;
        }
    }
</style>

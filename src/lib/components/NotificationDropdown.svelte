<script lang="ts">
    import { onMount } from "svelte";
    import { fly, fade } from "svelte/transition";
    import { clickOutside } from "$lib/actions/clickOutside";
    import Icon from "./Icon.svelte";
    import {
        notifications,
        unreadCount,
        loading,
        loadNotifications,
        markAsRead,
        markAllAsRead,
        deleteNotification,
    } from "$lib/stores/notifications";
    import { timeAgo } from "$lib/utils/date";
    import { goto } from "$app/navigation";
    import { user } from "$lib/stores/auth";

    let isOpen = $state(false);

    function toggle() {
        isOpen = !isOpen;
        if (isOpen) {
            // Refresh on open
            loadNotifications(1);
        }
    }

    function close() {
        isOpen = false;
    }

    function handleNotificationClick(n: any) {
        if (!n.is_read) {
            markAsRead(n.id);
        }
        if (n.action_url) {
            goto(n.action_url);
            close();
        }
    }

    function getIconForType(type: string) {
        switch (type) {
            case "success":
                return "check-circle";
            case "warning":
                return "alert-circle";
            case "error":
                return "alert-circle"; // or x-circle if available
            default:
                return "info";
        }
    }

    function getColorForType(type: string) {
        switch (type) {
            case "success":
                return "var(--color-success)";
            case "warning":
                return "var(--color-warning)";
            case "error":
                return "var(--color-danger)";
            default:
                return "var(--color-info)";
        }
    }
</script>

<div class="notification-dropdown" use:clickOutside={{ callback: close }}>
    <button class="icon-btn" onclick={toggle} title="Notifications">
        <Icon name="bell" size={18} />
        {#if $unreadCount > 0}
            <span class="badge-dot" transition:fade></span>
        {/if}
    </button>

    {#if isOpen}
        <div class="dropdown-panel" transition:fly={{ y: 10, duration: 200 }}>
            <div class="header">
                <h3>Notifications</h3>
                <div class="actions">
                    {#if $unreadCount > 0}
                        <button
                            class="text-btn"
                            onclick={() => markAllAsRead()}
                        >
                            Mark all read
                        </button>
                    {/if}
                    <button
                        class="icon-btn-sm"
                        onclick={() => {
                            const slug = $user?.tenant_slug || "default";
                            goto(`/${slug}/profile?tab=notifications`);
                            close();
                        }}
                        title="Settings"
                    >
                        <Icon name="settings" size={14} />
                    </button>
                </div>
            </div>

            <div class="content">
                {#if $loading && $notifications.length === 0}
                    <div class="loading">
                        <div class="spinner"></div>
                    </div>
                {:else if $notifications.length === 0}
                    <div class="empty-state">
                        <div class="icon-bg">
                            <Icon
                                name="bell"
                                size={24}
                                color="var(--text-tertiary)"
                            />
                        </div>
                        <p>No notifications yet</p>
                    </div>
                {:else}
                    <div class="list">
                        {#each $notifications as n (n.id)}
                            <div
                                class="notification-item"
                                class:unread={!n.is_read}
                                onclick={() => handleNotificationClick(n)}
                                role="button"
                                tabindex="0"
                                onkeydown={(e) =>
                                    e.key === "Enter" &&
                                    handleNotificationClick(n)}
                            >
                                <div class="icon-col">
                                    <Icon
                                        name={getIconForType(
                                            n.notification_type,
                                        )}
                                        size={18}
                                        color={getColorForType(
                                            n.notification_type,
                                        )}
                                    />
                                </div>
                                <div class="text-col">
                                    <p class="title">{n.title}</p>
                                    <p class="message">{n.message}</p>
                                    <span class="time"
                                        >{timeAgo(n.created_at)}</span
                                    >
                                </div>
                                <div class="actions-col">
                                    {#if !n.is_read}
                                        <button
                                            class="action-btn"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                markAsRead(n.id);
                                            }}
                                            title="Mark as read"
                                        >
                                            <div class="dot"></div>
                                        </button>
                                    {/if}
                                    <button
                                        class="delete-btn"
                                        onclick={(e) => {
                                            e.stopPropagation();
                                            deleteNotification(n.id);
                                        }}
                                        title="Delete"
                                    >
                                        <Icon name="x" size={12} />
                                    </button>
                                </div>
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>

            <div class="footer">
                <!-- <a href="/notifications" onclick={close}>View all</a> -->
            </div>
        </div>
    {/if}
</div>

<style>
    .notification-dropdown {
        position: relative;
    }

    .icon-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        width: 32px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 6px;
        cursor: pointer;
        position: relative;
        transition: all 0.2s;
    }

    .icon-btn:hover,
    :global(.icon-btn.active) {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .badge-dot {
        position: absolute;
        top: 6px;
        right: 6px;
        width: 8px;
        height: 8px;
        background: var(--color-danger);
        border-radius: 50%;
        border: 2px solid var(--bg-primary);
    }

    .dropdown-panel {
        position: absolute;
        top: 100%;
        right: -10px; /* Align slightly specific to topbar layout */
        margin-top: 8px;
        width: 360px;
        background: var(--bg-surface);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        box-shadow: 0 10px 40px -10px rgba(0, 0, 0, 0.2);
        z-index: 1000;
        display: flex;
        flex-direction: column;
        max-height: 80vh;
        overflow: hidden;
    }

    .header {
        padding: 1rem;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: var(--bg-hover);
    }

    .header h3 {
        margin: 0;
        font-size: 0.95rem;
        font-weight: 600;
        color: var(--text-primary);
    }

    .actions {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }

    .text-btn {
        background: none;
        border: none;
        color: var(--color-primary);
        font-size: 0.8rem;
        cursor: pointer;
        padding: 0;
    }

    .text-btn:hover {
        text-decoration: underline;
    }

    .icon-btn-sm {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        padding: 4px;
        border-radius: 4px;
        cursor: pointer;
        display: flex;
    }

    .icon-btn-sm:hover {
        background: var(--bg-tertiary);
        color: var(--text-primary);
    }

    .content {
        flex: 1;
        overflow-y: auto;
        min-height: 100px;
        max-height: 400px;
        background: var(--bg-surface);
    }

    .loading {
        display: flex;
        justify-content: center;
        align-items: center;
        padding: 2rem;
    }

    .spinner {
        width: 20px;
        height: 20px;
        border: 2px solid var(--border-color);
        border-top-color: var(--color-primary);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 3rem 1rem;
        color: var(--text-tertiary);
        gap: 0.5rem;
    }

    .icon-bg {
        width: 48px;
        height: 48px;
        background: var(--bg-hover);
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-bottom: 0.5rem;
    }

    .list {
        display: flex;
        flex-direction: column;
    }

    .notification-item {
        display: flex;
        padding: 1rem;
        gap: 0.8rem;
        border-bottom: 1px solid var(--border-color);
        cursor: pointer;
        transition: background 0.2s;
        position: relative;
    }

    .notification-item:hover {
        background: var(--bg-hover);
    }

    .notification-item.unread {
        background: var(--bg-active); /* Or slightly tinted */
    }

    .notification-item.unread:hover {
        background: var(--bg-tertiary);
    }

    .icon-col {
        padding-top: 2px;
        flex-shrink: 0;
    }

    .text-col {
        flex: 1;
        min-width: 0; /* Text truncation */
    }

    .title {
        font-size: 0.9rem;
        font-weight: 500;
        color: var(--text-primary);
        margin: 0 0 2px 0;
        line-height: 1.3;
    }

    .unread .title {
        font-weight: 600;
    }

    .message {
        font-size: 0.85rem;
        color: var(--text-secondary);
        margin: 0 0 4px 0;
        line-height: 1.4;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }

    .time {
        font-size: 0.75rem;
        color: var(--text-tertiary);
    }

    .actions-col {
        display: flex;
        flex-direction: column;
        align-items: flex-end;
        gap: 0.5rem;
        opacity: 0; /* Hide by default, show on hover */
        transition: opacity 0.2s;
    }

    .notification-item:hover .actions-col {
        opacity: 1;
    }

    /* Always show if unread? Maybe just the dot */
    .notification-item.unread .actions-col {
        opacity: 1;
    }

    .action-btn {
        background: none;
        border: none;
        cursor: pointer;
        padding: 4px;
    }

    .dot {
        width: 8px;
        height: 8px;
        background: var(--color-primary);
        border-radius: 50%;
    }

    .delete-btn {
        background: none;
        border: none;
        color: var(--text-tertiary);
        cursor: pointer;
        padding: 4px;
        border-radius: 4px;
        display: flex;
    }

    .delete-btn:hover {
        background: var(--bg-tertiary);
        color: var(--color-danger);
    }
</style>

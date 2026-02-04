<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';

  let {
    notificationCategories,
    preferences,
    pushEnabled,
    pushPermission = $bindable(),
    isDesktop,
    tenantPrefix,
    onUpdatePreference,
    onSubscribePush,
    onUnsubscribePush,
    onSendTestNotification,
    goto,
  } = $props();
</script>

<div class="card section fade-in-up">
  <div class="notifications-header">
    <div>
      <h2 class="section-title">
        {$t('profile.notifications.title') || 'Notification Preferences'}
      </h2>
      <p class="section-subtitle">
        {$t('profile.notifications.subtitle') || 'Customize how and when you want to be notified.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn btn-outline btn-sm" onclick={() => goto(`${tenantPrefix}/notifications`)}>
        <Icon name="bell" size={14} />
        <span>{$t('profile.notifications.view_all') || 'View all'}</span>
      </button>
      <button class="btn btn-outline btn-sm" onclick={onSendTestNotification}>
        <Icon name="bell" size={14} />
        <span>{$t('profile.notifications.test') || 'Test Notification'}</span>
      </button>
    </div>
  </div>

  <!-- Push Notification Banner / Status Card -->
  {#if !isDesktop}
    {#if pushEnabled}
      <div class="push-banner active">
        <div class="push-content">
          <div class="push-icon success">
            <Icon name="check" size={20} />
          </div>
          <div class="push-text">
            <h4>
              {$t('profile.notifications.push.active_title') || 'Push Notifications Active'}
            </h4>
            <p>
              {$t('profile.notifications.push.active_desc') ||
                'You are subscribed to real-time updates on this device.'}
            </p>
          </div>
        </div>
        <button class="btn btn-outline btn-sm" onclick={onUnsubscribePush}>
          {$t('profile.notifications.push.disable') || 'Disable'}
        </button>
      </div>
    {:else if pushPermission !== 'granted' || !pushEnabled}
      <!-- Show enable banner if not enabled -->
      <div class="push-banner">
        <div class="push-content">
          <div class="push-icon">
            <Icon name="bell" size={20} />
          </div>
          <div class="push-text">
            <h4>
              {$t('profile.notifications.push.enable_title') || 'Enable Push Notifications'}
            </h4>
            <p>
              {$t('profile.notifications.push.enable_desc') ||
                'Get real-time updates even when the app is closed.'}
            </p>
          </div>
        </div>
        <button
          class="btn btn-dark btn-sm"
          onclick={async () => {
            await onSubscribePush();
            pushPermission = Notification.permission;
          }}
        >
          {$t('profile.notifications.push.enable') || 'Enable Push'}
        </button>
      </div>
    {/if}
  {/if}

  <div class="prefs-grid">
    {#each notificationCategories as category}
      <div class="pref-card">
        <div class="pref-card-header">
          <div class="cat-icon {category.id}">
            <Icon name={category.icon} size={18} />
          </div>
          <div class="cat-info">
            <h3>{category.label}</h3>
            <p>{category.desc}</p>
          </div>
        </div>

        <div class="pref-channels">
          {#each ['in_app', 'email', 'push'] as channel}
            {@const pref = preferences.find(
              (p: any) => p.category === category.id && p.channel === channel,
            )}
            {@const isDisabled = category.id === 'security' && channel === 'email'}
            {@const isEnabled =
              category.id === 'security' && channel === 'email'
                ? true
                : (pref?.enabled ?? channel === 'in_app')}

            <label class="channel-row" class:disabled={isDisabled}>
              <div class="channel-info">
                <span class="channel-name">
                  {channel === 'in_app'
                    ? $t('profile.notifications.channels.in_app') || 'In-App'
                    : channel === 'email'
                      ? $t('profile.notifications.channels.email') || 'Email'
                      : $t('profile.notifications.channels.push') || 'Push'}
                </span>
                {#if isDisabled}
                  <span class="channel-note"
                    >{$t('profile.notifications.channels.required') || 'Required'}</span
                  >
                {/if}
              </div>

              <div class="switch">
                <input
                  type="checkbox"
                  checked={isEnabled}
                  disabled={isDisabled}
                  onchange={(e) =>
                    onUpdatePreference(channel, category.id, e.currentTarget.checked)}
                />
                <span class="slider round"></span>
              </div>
            </label>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 2rem;
    box-shadow: var(--shadow-sm);
  }

  .notifications-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 2rem;
    gap: 1rem;
  }

  .header-actions {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .section-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 0.5rem 0;
  }

  .section-subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
  }

  /* Push Banner */
  .push-banner {
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-left: 4px solid var(--color-primary);
    border-radius: var(--radius-md);
    padding: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1.5rem;
    margin-bottom: 2rem;
  }

  .push-content {
    display: flex;
    align-items: center;
    gap: 1.25rem;
  }

  .push-icon {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--bg-surface);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-primary);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
  }

  .push-text h4 {
    margin: 0 0 0.25rem 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .push-text p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .push-banner.active {
    border-left-color: var(--color-success);
    background: rgba(16, 185, 129, 0.05);
  }

  .push-icon.success {
    background: rgba(16, 185, 129, 0.1);
    color: var(--color-success);
  }

  /* Preferences Grid */
  .prefs-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1.5rem;
  }

  .pref-card {
    background: var(--bg-app);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .pref-card-header {
    padding: 1.25rem;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 1rem;
    background: var(--bg-surface);
  }

  .cat-icon {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }

  .cat-icon.system {
    background: linear-gradient(135deg, #3b82f6, #2563eb);
  }
  .cat-icon.team {
    background: linear-gradient(135deg, #10b981, #059669);
  }
  .cat-icon.payment {
    background: linear-gradient(135deg, #f59e0b, #d97706);
  }
  .cat-icon.security {
    background: linear-gradient(135deg, #ef4444, #dc2626);
  }

  .cat-info h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .cat-info p {
    margin: 0;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .pref-channels {
    padding: 0.5rem 0;
  }

  .channel-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.85rem 1.25rem;
    cursor: pointer;
    transition: background 0.1s;
  }

  .channel-row:hover:not(.disabled) {
    background: var(--bg-hover);
  }

  .channel-row.disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .channel-info {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }

  .channel-name {
    font-weight: 500;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .channel-note {
    font-size: 0.75rem;
    font-weight: 800;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    border: 1px solid rgba(99, 102, 241, 0.35);
    background: rgba(99, 102, 241, 0.12);
    color: var(--text-primary);
    white-space: nowrap;
  }

  /* Switch and Buttons */
  .switch {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--bg-active);
    transition: 0.3s;
    border: 1px solid var(--border-input);
  }

  .slider:before {
    position: absolute;
    content: '';
    height: 14px;
    width: 14px;
    left: 2px;
    bottom: 2px;
    background-color: white;
    transition: 0.3s;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  input:checked + .slider {
    background-color: var(--color-primary);
    border-color: var(--color-primary);
  }

  input:checked + .slider:before {
    transform: translateX(16px);
  }

  .slider.round {
    border-radius: 34px;
  }

  .slider.round:before {
    border-radius: 50%;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.6rem 1.25rem;
    border-radius: var(--radius-md);
    font-weight: 500;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
    border: 1px solid transparent;
  }

  .btn-outline {
    background: transparent;
    border-color: var(--border-color);
    color: var(--text-primary);
  }

  .btn-outline:hover {
    background: var(--bg-hover);
  }

  .btn-dark {
    background: var(--text-primary);
    color: var(--bg-app);
    border: none;
  }

  .btn-dark:hover {
    opacity: 0.9;
  }

  .btn-sm {
    padding: 0.5rem 1rem;
    font-size: 0.85rem;
  }

  @media (max-width: 600px) {
    .notifications-header {
      flex-direction: column;
      align-items: flex-start;
      margin-bottom: 1.25rem;
    }

    .header-actions {
      width: 100%;
      justify-content: flex-start;
    }

    .prefs-grid {
      grid-template-columns: 1fr;
    }

    .push-banner {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .push-content {
      gap: 1rem;
    }
  }

  .fade-in-up {
    animation: fadeInUp 0.4s ease-out;
  }

  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>

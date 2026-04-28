<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import {
    getNotificationChannelReadiness,
    parseWhatsAppEventPreferences,
    sanitizeWhatsAppEventPreference,
    serializeWhatsAppEventPreferences,
    type NotificationChannelReadiness,
  } from '$lib/utils/whatsappGateway';
  import type {
    WhatsAppEventDefinition,
    WhatsAppEventPreference,
    WhatsAppEventScope,
  } from '$lib/api/types';

  let {
    localSettings,
    handleChange,
    eventSettingsKey,
    eventScope,
    emailReady,
    emailReadinessReason,
    title = 'Event Notifications',
    description = 'Choose channels for each notification event.',
  }: {
    localSettings: Record<string, string>;
    handleChange: (key: string, value: string | boolean) => void;
    eventSettingsKey: 'wa_events_platform' | 'wa_events_tenant';
    eventScope: WhatsAppEventScope;
    emailReady?: boolean;
    emailReadinessReason?: string | null;
    title?: string;
    description?: string;
  } = $props();

  type Channel = keyof WhatsAppEventPreference;

  let events = $state<WhatsAppEventDefinition[]>([]);
  let loadingEvents = $state(false);
  let eventsError = $state('');

  const scopedEvents = $derived(events.filter((event) => event.scope === eventScope));
  const eventPreferences = $derived(
    parseWhatsAppEventPreferences(localSettings[eventSettingsKey], scopedEvents),
  );
  const channelReadiness = $derived.by<NotificationChannelReadiness>(() => {
    const readiness = getNotificationChannelReadiness(localSettings);
    if (typeof emailReady === 'boolean') {
      readiness.email = {
        ready: emailReady,
        reason: emailReady
          ? null
          : emailReadinessReason || 'Configure email provider first.',
      };
    }
    return readiness;
  });

  onMount(() => {
    void loadEvents();
  });

  $effect(() => {
    if (scopedEvents.length === 0) return;

    let changed = false;
    const sanitized = Object.fromEntries(
      scopedEvents.map((event) => {
        const current = eventPreferences[event.code];
        const next = sanitizeWhatsAppEventPreference(current, channelReadiness);
        if (
          current.whatsapp !== next.whatsapp ||
          current.email !== next.email ||
          current.in_app !== next.in_app
        ) {
          changed = true;
        }
        return [event.code, next];
      }),
    );
    if (!changed) return;

    const serialized = serializeWhatsAppEventPreferences(sanitized);
    handleChange(eventSettingsKey, serialized);
  });

  async function loadEvents() {
    loadingEvents = true;
    eventsError = '';
    try {
      events = await api.whatsapp.listEvents();
    } catch (error: any) {
      eventsError = error?.message || 'Failed to load notification events';
    } finally {
      loadingEvents = false;
    }
  }

  function channelReady(channel: Channel): boolean {
    return channelReadiness[channel].ready;
  }

  function channelReason(channel: Channel): string | null {
    return channelReadiness[channel].reason;
  }

  function updateEventPreference(eventCode: string, channel: Channel, checked: boolean) {
    if (checked && !channelReady(channel)) return;

    const nextPreference = sanitizeWhatsAppEventPreference(
      {
        ...(eventPreferences[eventCode] || { whatsapp: false, email: false, in_app: false }),
        [channel]: checked,
      },
      channelReadiness,
    );
    const next = {
      ...eventPreferences,
      [eventCode]: nextPreference,
    };
    handleChange(eventSettingsKey, serializeWhatsAppEventPreferences(next));
  }
</script>

<div class="notification-events">
  <div class="config-panel fade-in">
    <div class="panel-heading">
      <div>
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
      <div class="channel-status">
        <span class:ready={channelReadiness.whatsapp.ready}>
          <Icon name="message-circle" size={14} />
          WhatsApp
        </span>
        <span class:ready={channelReadiness.email.ready}>
          <Icon name="mail" size={14} />
          Email
        </span>
        <span class="ready">
          <Icon name="bell" size={14} />
          In App
        </span>
      </div>
    </div>

    {#if !channelReadiness.whatsapp.ready || !channelReadiness.email.ready}
      <div class="readiness-notice">
        <Icon name="info" size={16} />
        <div>
          {#if !channelReadiness.whatsapp.ready}
            <span>{channelReadiness.whatsapp.reason}</span>
          {/if}
          {#if !channelReadiness.email.ready}
            <span>{channelReadiness.email.reason}</span>
          {/if}
        </div>
      </div>
    {/if}

    {#if loadingEvents}
      <div class="inline-loading">
        <div class="spinner"></div>
      </div>
    {:else if eventsError}
      <div class="status error">
        <span>{eventsError}</span>
        <button class="btn btn-secondary btn-sm" type="button" onclick={loadEvents}>Retry</button>
      </div>
    {:else if scopedEvents.length === 0}
      <p class="help-text">No notification events are available for this scope yet.</p>
    {:else}
      <div class="event-table">
        <div class="event-row header">
          <span>Event</span>
          <span>WhatsApp</span>
          <span>Email</span>
          <span>In App</span>
        </div>
        {#each scopedEvents as event}
          {@const pref = eventPreferences[event.code] || { whatsapp: false, email: false, in_app: false }}
          <div class="event-row">
            <div class="event-info">
              <strong>{event.label}</strong>
              <small>{event.description || event.code}</small>
            </div>
            {#each ['whatsapp', 'email', 'in_app'] as channel}
              {@const typedChannel = channel as Channel}
              <label
                class="mini-toggle"
                class:disabled={!channelReady(typedChannel)}
                title={channelReason(typedChannel) || undefined}
              >
                <input
                  type="checkbox"
                  checked={channelReady(typedChannel) && pref[typedChannel]}
                  disabled={!channelReady(typedChannel)}
                  onchange={(changeEvent) =>
                    updateEventPreference(
                      event.code,
                      typedChannel,
                      changeEvent.currentTarget.checked,
                    )}
                />
                <span></span>
              </label>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .notification-events {
    display: flex;
    flex-direction: column;
  }

  .panel-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .panel-heading h3 {
    margin: 0 0 0.35rem;
    color: var(--text-primary);
    font-size: 1rem;
    font-weight: 650;
  }

  .panel-heading p,
  .help-text {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.875rem;
    line-height: 1.5;
  }

  .channel-status {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .channel-status span {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    min-height: 28px;
    padding: 0 0.65rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
  }

  .channel-status span.ready {
    border-color: var(--color-success);
    color: var(--color-success);
    background: color-mix(in srgb, var(--color-success) 10%, transparent);
  }

  .readiness-notice {
    display: flex;
    align-items: flex-start;
    gap: 0.65rem;
    margin-bottom: 1rem;
    padding: 0.85rem 1rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 0.875rem;
  }

  .readiness-notice > div {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .event-table {
    overflow: hidden;
    margin-top: 1rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
  }

  .event-row {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) repeat(3, 92px);
    align-items: center;
    gap: 0.75rem;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .event-row:last-child {
    border-bottom: none;
  }

  .event-row.header {
    background: var(--bg-app);
    color: var(--text-secondary);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .event-info {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.2rem;
  }

  .event-info strong {
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .event-info small {
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.35;
  }

  .mini-toggle {
    display: flex;
    justify-content: center;
  }

  .mini-toggle input {
    position: absolute;
    opacity: 0;
  }

  .mini-toggle span {
    width: 34px;
    height: 20px;
    border-radius: 999px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    cursor: pointer;
    position: relative;
    transition: background 0.2s;
  }

  .mini-toggle.disabled span {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .mini-toggle span::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition:
      transform 0.2s,
      background 0.2s;
  }

  .mini-toggle input:checked + span {
    background: var(--color-primary-subtle);
    border-color: var(--color-primary);
  }

  .mini-toggle input:checked + span::after {
    transform: translateX(14px);
    background: var(--color-primary);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
  }

  .status.error {
    color: var(--color-danger);
  }

  .inline-loading {
    display: flex;
    justify-content: center;
    padding: 1.5rem;
  }

  @media (max-width: 760px) {
    .panel-heading {
      flex-direction: column;
    }

    .channel-status {
      justify-content: flex-start;
    }

    .event-row {
      grid-template-columns: minmax(160px, 1fr) repeat(3, 64px);
      padding: 0.75rem;
    }
  }
</style>

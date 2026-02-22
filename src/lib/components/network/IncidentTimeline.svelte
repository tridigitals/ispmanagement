<script lang="ts">
  import { t } from 'svelte-i18n';

  type ActivityItem = {
    ts: string;
    title: string;
    detail?: string;
  };

  let {
    items = [],
  }: {
    items?: ActivityItem[];
  } = $props();
</script>

<div class="timeline">
  <div class="timeline-title">
    {$t('admin.network.incidents.activity.title') || 'Activity Timeline'}
  </div>
  <div class="timeline-list">
    {#each items as event}
      <div class="timeline-item">
        <span class="dot"></span>
        <div class="timeline-content">
          <div class="timeline-row">
            <span class="timeline-event">{event.title}</span>
            <span class="timeline-time">{event.ts}</span>
          </div>
          {#if event.detail}
            <div class="timeline-detail">{event.detail}</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .timeline {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
    display: grid;
    gap: 10px;
  }
  .timeline-title {
    font-size: 0.8rem;
    font-weight: 800;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .timeline-list {
    display: grid;
    gap: 10px;
  }
  .timeline-item {
    display: grid;
    grid-template-columns: 14px 1fr;
    gap: 8px;
    align-items: start;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--accent);
    margin-top: 6px;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .timeline-content {
    display: grid;
    gap: 3px;
  }
  .timeline-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .timeline-event {
    color: var(--text-primary);
    font-weight: 700;
    font-size: 0.88rem;
  }
  .timeline-time {
    color: var(--text-secondary);
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .timeline-detail {
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.4;
  }
</style>

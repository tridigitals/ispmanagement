<script lang="ts">
  import Icon from '../ui/Icon.svelte';

  let { title, value, icon, color = 'primary', trend = null, trendType = 'neutral' } = $props();

  function getColorVar(c: string) {
    switch (c) {
      case 'success':
        return 'var(--color-success)';
      case 'warning':
        return 'var(--color-warning)';
      case 'danger':
        return 'var(--color-danger)';
      default:
        return 'var(--color-primary)';
    }
  }
</script>

<div class="stats-card">
  <div class="icon-wrapper" style="background: {getColorVar(color)}20; color: {getColorVar(color)}">
    <Icon name={icon} size={24} />
  </div>
  <div class="content">
    <div class="title">{title}</div>
    <div class="value">{value}</div>
    {#if trend}
      <div class="trend {trendType}">
        <Icon
          name={trendType === 'up'
            ? 'trending-up'
            : trendType === 'down'
              ? 'trending-down'
              : 'minus'}
          size={14}
        />
        <span>{trend}</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .stats-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.25rem 1.25rem;
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    transition:
      background 0.2s,
      border-color 0.2s;
  }

  .stats-card:hover {
    background: var(--bg-secondary);
    border-color: color-mix(in srgb, var(--color-primary) 24%, var(--border-color));
  }

  .icon-wrapper {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .content {
    flex: 1;
  }

  .title {
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin-bottom: 0.25rem;
  }

  .value {
    color: var(--text-primary);
    font-size: 1.5rem;
    font-weight: 700;
    line-height: 1.2;
  }

  .trend {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.8rem;
    margin-top: 0.5rem;
    font-weight: 600;
  }

  .trend.up {
    color: var(--color-success);
  }
  .trend.down {
    color: var(--color-danger);
  }
  .trend.neutral {
    color: var(--text-secondary);
  }

</style>

<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  type ActionHandler = (() => void | Promise<void>) | undefined;

  let {
    fullWidth = false,
    showOpen = true,
    showDetail = false,
    showSnooze = false,
    showAcknowledge = false,
    showResolve = false,
    onOpen,
    onDetail,
    onSnooze,
    onAcknowledge,
    onResolve,
  }: {
    fullWidth?: boolean;
    showOpen?: boolean;
    showDetail?: boolean;
    showSnooze?: boolean;
    showAcknowledge?: boolean;
    showResolve?: boolean;
    onOpen?: ActionHandler;
    onDetail?: ActionHandler;
    onSnooze?: ActionHandler;
    onAcknowledge?: ActionHandler;
    onResolve?: ActionHandler;
  } = $props();
</script>

<div class="row-actions" class:full-width={fullWidth}>
  {#if showOpen}
    <button class="row-action-btn" type="button" onclick={() => void onOpen?.()} title={$t('common.open') || 'Open'}>
      <Icon name="arrow-right" size={16} />
    </button>
  {/if}

  {#if showDetail}
    <button class="row-action-btn" type="button" onclick={() => void onDetail?.()} title={$t('common.details') || 'Details'}>
      <Icon name="file-text" size={16} />
    </button>
  {/if}

  {#if showSnooze}
    <button
      class="row-action-btn"
      type="button"
      onclick={() => void onSnooze?.()}
      title={$t('admin.network.alerts.actions.snooze_30m') || 'Snooze 30m'}
    >
      <Icon name="clock" size={16} />
    </button>
  {/if}

  {#if showAcknowledge}
    <button
      class="row-action-btn"
      type="button"
      onclick={() => void onAcknowledge?.()}
      title={$t('admin.network.alerts.actions.ack') || 'Acknowledge'}
    >
      <Icon name="check" size={16} />
    </button>
  {/if}

  {#if showResolve}
    <button
      class="row-action-btn"
      type="button"
      onclick={() => void onResolve?.()}
      title={$t('admin.network.alerts.actions.resolve') || 'Resolve'}
    >
      <Icon name="check-circle" size={16} />
    </button>
  {/if}
</div>

<style>
  .row-actions {
    display: inline-flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .row-actions.full-width {
    width: 100%;
  }

  .row-action-btn {
    width: 34px;
    height: 34px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    border-radius: 10px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .row-action-btn:hover {
    background: var(--bg-hover);
  }
</style>

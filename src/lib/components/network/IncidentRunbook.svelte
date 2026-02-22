<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  type RunbookStep = {
    title: string;
    detail?: string;
    command?: string;
  };

  let {
    steps = [],
    canManage = false,
    onCopyCommand,
    onAddStep,
  }: {
    steps?: RunbookStep[];
    canManage?: boolean;
    onCopyCommand: (command: string) => void | Promise<void>;
    onAddStep: (step: RunbookStep) => void;
  } = $props();
</script>

<div class="runbook">
  <div class="runbook-title">
    {$t('admin.network.incidents.runbook.title') || 'What to do next'}
  </div>
  <div class="runbook-sub">
    {$t('admin.network.incidents.runbook.subtitle') || 'Operator checklist based on incident type.'}
  </div>
  <div class="runbook-list">
    {#each steps as step}
      <div class="runbook-item">
        <div class="runbook-text">
          <div class="runbook-step">{step.title}</div>
          {#if step.detail}
            <div class="runbook-detail">{step.detail}</div>
          {/if}
          {#if step.command}
            <code class="runbook-command">{step.command}</code>
          {/if}
        </div>
        <div class="runbook-actions">
          {#if step.command}
            <button class="icon-btn" type="button" onclick={() => void onCopyCommand(step.command!)}>
              <Icon name="link" size={14} />
            </button>
          {/if}
          {#if canManage}
            <button class="icon-btn" type="button" onclick={() => onAddStep(step)}>
              <Icon name="check" size={14} />
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .runbook {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 12px;
    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
    display: grid;
    gap: 8px;
  }
  .runbook-title {
    font-size: 0.82rem;
    font-weight: 800;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .runbook-sub {
    color: var(--text-secondary);
    font-size: 0.8rem;
  }
  .runbook-list {
    display: grid;
    gap: 8px;
  }
  .runbook-item {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 10px;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 10px;
    align-items: start;
  }
  .runbook-step {
    color: var(--text-primary);
    font-weight: 700;
    font-size: 0.86rem;
  }
  .runbook-detail {
    color: var(--text-secondary);
    font-size: 0.8rem;
    margin-top: 4px;
  }
  .runbook-command {
    display: inline-block;
    margin-top: 6px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: color-mix(in srgb, var(--bg-surface) 75%, transparent);
    padding: 4px 8px;
    color: var(--text-primary);
    font-size: 0.75rem;
  }
  .runbook-actions {
    display: inline-flex;
    gap: 6px;
  }
  .icon-btn {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
  }
</style>

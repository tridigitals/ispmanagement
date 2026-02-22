<script lang="ts">
  let {
    title,
    subtitle = '',
    actions,
  }: {
    title: string;
    subtitle?: string;
    actions?: import('svelte').Snippet;
  } = $props();
</script>

<div class="network-page-header">
  <div class="network-page-heading">
    <h1 class="network-page-title">{title}</h1>
    {#if subtitle}
      <p class="network-page-subtitle">{subtitle}</p>
    {/if}
  </div>

  <div class="network-page-actions">
    {#if actions}
      {@render actions()}
    {/if}
  </div>
</div>

<style>
  .network-page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }

  .network-page-title {
    margin: 0;
    font-size: 1.6rem;
    font-weight: 950;
    color: var(--text-primary);
  }

  .network-page-subtitle {
    margin: 0.35rem 0 0 0;
    color: var(--text-secondary);
  }

  .network-page-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
</style>

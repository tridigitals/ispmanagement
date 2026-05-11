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
    gap: 10px 14px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .network-page-heading {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .network-page-title {
    margin: 0;
    font-size: clamp(1.22rem, 1rem + 0.75vw, 1.48rem);
    font-weight: 950;
    color: var(--text-primary);
    line-height: 1.1;
  }

  .network-page-subtitle {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.4;
  }

  .network-page-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }

  @media (max-width: 768px) {
    .network-page-header {
      margin-bottom: 10px;
    }

    .network-page-actions {
      width: 100%;
      justify-content: stretch;
    }

    .network-page-actions :global(.btn),
    .network-page-actions :global(a.btn) {
      flex: 1 1 calc(50% - 0.4rem);
      justify-content: center;
      min-width: 0;
    }
  }
</style>

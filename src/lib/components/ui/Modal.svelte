<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import Icon from './Icon.svelte';

  let {
    title = '',
    width = '420px',
    show = $bindable(false),
    bodyOverflow = 'auto',
    onclose,
    children,
    footer,
  } = $props<{
    title?: string;
    width?: string;
    show: boolean;
    bodyOverflow?: 'auto' | 'visible';
    onclose?: () => void;
    children?: import('svelte').Snippet;
    footer?: import('svelte').Snippet;
  }>();

  let backdropPointerDown = $state(false);

  function close() {
    show = false;
    if (onclose) onclose();
  }

  function handleBackdropPointerDown(event: PointerEvent) {
    backdropPointerDown = event.target === event.currentTarget;
  }

  function handleBackdropClick(event: MouseEvent) {
    const isDirectBackdropClick = event.target === event.currentTarget;
    if (backdropPointerDown && isDirectBackdropClick) {
      close();
    }
    backdropPointerDown = false;
  }
</script>

{#if show}
  <div
    class="modal-backdrop"
    onpointerdown={handleBackdropPointerDown}
    onclick={handleBackdropClick}
    onkeydown={(e) => e.key === 'Escape' && close()}
    role="button"
    tabindex="0"
    transition:fade={{ duration: 200 }}
  >
    <div
      class="modal-card"
      style="max-width: {width}"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      transition:fly={{ y: 20, duration: 300 }}
    >
      <div class="modal-header">
        <h3>{title}</h3>
        <button class="close-btn" onclick={close}>
          <Icon name="x" size={20} />
        </button>
      </div>
      <div class="modal-body" class:body-overflow-visible={bodyOverflow === 'visible'}>
        {#if children}
          {@render children()}
        {/if}
      </div>
      {#if footer}
        <div class="modal-footer">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.7);
        display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
  }

  .modal-card {
    background: var(--bg-surface, #1e293b);
    /* Teks EKSPLISIT, bukan warisan body: `body { color:
       var(--text-primary) }` di global.css di-resolve terhadap :root gelap
       (#f2f4f8), dan modal fixed di luar .v2-light mewarisinya -> teks
       nyaris putih di atas kartu putih (input file cover, placeholder).
       Di legacy: var = terang di atas kartu gelap, tetap benar. */
    color: var(--text-primary, #f2f4f8);
    width: 100%;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    box-shadow: var(--shadow-md);
    display: flex;
    flex-direction: column;
    max-height: 90vh;
  }

  .modal-header {
    padding: 1.25rem 1.25rem 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary, white);
    font-weight: 600;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary, #94a3b8);
    cursor: pointer;
    padding: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    transition: all 0.2s;
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary, #fff);
  }

  .modal-body {
    padding: 1.25rem;
    overflow-y: auto;
  }

  .modal-body.body-overflow-visible {
    overflow: visible;
  }

  .modal-footer {
    padding: 1rem 1.25rem 1.25rem;
    display: flex;
    justify-content: flex-end;
    gap: 1rem;
    border-top: 1px solid var(--border-color, rgba(255, 255, 255, 0.05));
    background: transparent; /* Explicitly transparent */
    border-bottom-left-radius: var(--radius-lg);
    border-bottom-right-radius: var(--radius-lg);
  }

  @media (max-width: 640px) {
    .modal-card {
      max-width: 100% !important;
      margin: 0;
      max-height: calc(100dvh - 2rem);
    }

    .modal-backdrop {
      align-items: center;
      padding: 1rem;
    }

    .modal-body {
      max-height: 70vh;
    }
  }
</style>

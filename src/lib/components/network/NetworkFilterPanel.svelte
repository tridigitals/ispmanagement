<div class="network-filter-panel">
  <slot />
</div>

<style>
  .network-filter-panel {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border-color);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-card) 80%, transparent);
  }

  :global(.network-filter-panel .control) {
    display: grid;
    gap: 6px;
  }

  :global(.network-filter-panel .control label),
  :global(.network-filter-panel .control .label) {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-weight: 700;
  }

  :global(.network-filter-panel .control-spacer) {
    height: 1rem;
  }

  :global(.network-filter-panel .input),
  :global(.network-filter-panel select),
  :global(.network-filter-panel input[type='date']) {
    height: 38px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 0 10px;
    outline: none;
  }

  :global(.network-filter-panel .input:focus),
  :global(.network-filter-panel select:focus),
  :global(.network-filter-panel input[type='date']:focus) {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border-color));
  }

  :global(.network-filter-panel .control-actions .btn) {
    width: 100%;
    justify-content: center;
    height: 38px;
    padding: 0 10px;
  }
</style>

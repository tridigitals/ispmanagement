<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { sanitizeHtml, stripHtmlToText } from '$lib/utils/sanitizeHtml';

  let {
    value = $bindable(''),
    label,
    placeholder = '',
    help,
    disabled = false,
    minHeight = 170,
  }: {
    value?: string;
    label?: string;
    placeholder?: string;
    help?: string;
    disabled?: boolean;
    minHeight?: number;
  } = $props();

  let editorEl: HTMLDivElement | null = $state(null);
  let isFocused = $state(false);

  $effect(() => {
    if (!editorEl) return;
    const next = sanitizeHtml(value || '');
    // Avoid resetting selection unnecessarily.
    if ((editorEl.innerHTML || '') !== next) {
      editorEl.innerHTML = next || '';
    }
  });

  function exec(cmd: string, arg?: string) {
    if (disabled) return;
    editorEl?.focus();
    // eslint-disable-next-line deprecation/deprecation
    document.execCommand(cmd, false, arg);
    syncValue();
  }

  function syncValue() {
    if (!editorEl) return;
    value = sanitizeHtml(editorEl.innerHTML || '');
  }

  function onInput() {
    syncValue();
  }

  function addLink() {
    if (disabled) return;
    const url = prompt('Link URL (https://...)');
    if (!url) return;
    exec('createLink', url);
  }

  function clearFormat() {
    exec('removeFormat');
    exec('unlink');
  }

  function hasText() {
    return stripHtmlToText(value || '').length > 0;
  }
</script>

<div class="wrap" class:disabled>
  {#if label}
    <div class="label">{label}</div>
  {/if}

  <div class="toolbar" role="toolbar" aria-label="Rich text toolbar">
    <button class="tool" type="button" onclick={() => exec('bold')} title="Bold">
      <Icon name="bold" size={16} />
    </button>
    <button class="tool" type="button" onclick={() => exec('italic')} title="Italic">
      <Icon name="italic" size={16} />
    </button>
    <button class="tool" type="button" onclick={() => exec('underline')} title="Underline">
      <Icon name="underline" size={16} />
    </button>
    <span class="sep"></span>
    <button class="tool" type="button" onclick={() => exec('insertUnorderedList')} title="Bullet list">
      <Icon name="list" size={16} />
    </button>
    <button class="tool" type="button" onclick={() => exec('insertOrderedList')} title="Numbered list">
      <Icon name="list-ordered" size={16} />
    </button>
    <button class="tool" type="button" onclick={() => exec('formatBlock', 'blockquote')} title="Quote">
      <Icon name="quote" size={16} />
    </button>
    <span class="sep"></span>
    <button class="tool" type="button" onclick={addLink} title="Link">
      <Icon name="link" size={16} />
    </button>
    <button class="tool" type="button" onclick={clearFormat} title="Clear formatting">
      <Icon name="eraser" size={16} />
    </button>
  </div>

  <div
    class="editor"
    bind:this={editorEl}
    contenteditable={!disabled}
    data-placeholder={placeholder}
    oninput={onInput}
    onfocus={() => (isFocused = true)}
    onblur={() => (isFocused = false)}
    style={`min-height: ${minHeight}px;`}
  ></div>

  {#if help}
    <div class="help">{help}</div>
  {/if}

  {#if !hasText() && !isFocused && placeholder}
    <!-- Placeholder handled via CSS, but keep this check to avoid lint issues in some webviews -->
  {/if}
</div>

<style>
  .wrap {
    display: grid;
    gap: 0.45rem;
  }

  .label {
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.9rem;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.4rem 0.45rem;
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.02);
  }

  :global([data-theme='light']) .toolbar {
    background: rgba(0, 0, 0, 0.01);
  }

  .tool {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-primary);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  :global([data-theme='light']) .tool {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.03);
  }

  .tool:hover {
    border-color: rgba(99, 102, 241, 0.45);
  }

  .sep {
    width: 1px;
    height: 22px;
    background: rgba(255, 255, 255, 0.12);
    margin: 0 0.2rem;
  }

  :global([data-theme='light']) .sep {
    background: rgba(0, 0, 0, 0.12);
  }

  .editor {
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 0.75rem 0.85rem;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.95rem;
    line-height: 1.6;
    outline: none;
    overflow: auto;
    white-space: normal;
  }

  .editor:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .editor:empty:before {
    content: attr(data-placeholder);
    color: rgba(255, 255, 255, 0.45);
    font-weight: 650;
  }

  :global([data-theme='light']) .editor:empty:before {
    color: rgba(0, 0, 0, 0.38);
  }

  .help {
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 650;
  }

  .disabled {
    opacity: 0.65;
    pointer-events: none;
  }
</style>


<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Editor } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Link from '@tiptap/extension-link';
  import Underline from '@tiptap/extension-underline';
  import Placeholder from '@tiptap/extension-placeholder';
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

  let editorHost: HTMLDivElement | null = $state(null);
  let editor: Editor | null = $state(null);
  let settingFromOutside = false;

  function currentHtml() {
    return sanitizeHtml(value || '');
  }

  function setFromOutside(html: string) {
    if (!editor) return;
    settingFromOutside = true;
    editor.commands.setContent(html || '', false);
    settingFromOutside = false;
  }

  function updateValueFromEditor() {
    if (!editor) return;
    const html = sanitizeHtml(editor.getHTML());
    value = html;
  }

  function promptLink() {
    if (!editor) return;
    const prev = editor.getAttributes('link')?.href || '';
    const url = prompt('Link URL (https://...)', prev);
    if (url === null) return;
    const next = (url || '').trim();
    if (!next) {
      editor.chain().focus().extendMarkRange('link').unsetLink().run();
      updateValueFromEditor();
      return;
    }
    editor
      .chain()
      .focus()
      .extendMarkRange('link')
      .setLink({ href: next, target: '_blank', rel: 'nofollow noopener noreferrer' })
      .run();
    updateValueFromEditor();
  }

  function canPublishHint() {
    return stripHtmlToText(value || '').length > 0;
  }

  onMount(() => {
    if (!editorHost) return;
    editor = new Editor({
      element: editorHost,
      editable: !disabled,
      content: currentHtml(),
      extensions: [
        StarterKit.configure({
          heading: { levels: [1, 2, 3] },
        }),
        Underline,
        Link.configure({
          openOnClick: true,
          autolink: true,
          linkOnPaste: true,
          HTMLAttributes: {
            rel: 'nofollow noopener noreferrer',
            target: '_blank',
          },
        }),
        Placeholder.configure({
          placeholder: placeholder || '',
        }),
      ],
      onUpdate: () => {
        if (settingFromOutside) return;
        updateValueFromEditor();
      },
    });

    return () => {
      editor?.destroy();
      editor = null;
    };
  });

  onDestroy(() => {
    editor?.destroy();
    editor = null;
  });

  // Keep editor content in sync when `value` changes externally.
  $effect(() => {
    if (!editor) return;
    const next = currentHtml();
    const cur = sanitizeHtml(editor.getHTML());
    if (cur !== next) setFromOutside(next);
  });

  $effect(() => {
    if (!editor) return;
    editor.setEditable(!disabled);
  });
</script>

<div class="wrap" class:disabled>
  {#if label}
    <div class="label">{label}</div>
  {/if}

  <div class="toolbar" role="toolbar" aria-label="Rich text toolbar">
    <button
      class="tool"
      class:active={!!editor?.isActive('bold')}
      type="button"
      onclick={() => editor?.chain().focus().toggleBold().run()}
      title="Bold"
    >
      <Icon name="bold" size={16} />
    </button>
    <button
      class="tool"
      class:active={!!editor?.isActive('italic')}
      type="button"
      onclick={() => editor?.chain().focus().toggleItalic().run()}
      title="Italic"
    >
      <Icon name="italic" size={16} />
    </button>
    <button
      class="tool"
      class:active={!!editor?.isActive('underline')}
      type="button"
      onclick={() => editor?.chain().focus().toggleUnderline().run()}
      title="Underline"
    >
      <Icon name="underline" size={16} />
    </button>
    <span class="sep"></span>
    <button
      class="tool"
      class:active={!!editor?.isActive('bulletList')}
      type="button"
      onclick={() => editor?.chain().focus().toggleBulletList().run()}
      title="Bullet list"
    >
      <Icon name="list" size={16} />
    </button>
    <button
      class="tool"
      class:active={!!editor?.isActive('orderedList')}
      type="button"
      onclick={() => editor?.chain().focus().toggleOrderedList().run()}
      title="Numbered list"
    >
      <Icon name="list-ordered" size={16} />
    </button>
    <button
      class="tool"
      class:active={!!editor?.isActive('blockquote')}
      type="button"
      onclick={() => editor?.chain().focus().toggleBlockquote().run()}
      title="Quote"
    >
      <Icon name="quote" size={16} />
    </button>
    <span class="sep"></span>
    <button class="tool" type="button" onclick={promptLink} title="Link">
      <Icon name="link" size={16} />
    </button>
    <button
      class="tool"
      type="button"
      onclick={() => editor?.chain().focus().unsetAllMarks().clearNodes().run()}
      title="Clear formatting"
    >
      <Icon name="eraser" size={16} />
    </button>
  </div>

  <div class="editor-shell" style={`min-height: ${minHeight}px;`}>
    <div class="editor" bind:this={editorHost}></div>
  </div>

  {#if help}
    <div class="help">{help}</div>
  {:else if !canPublishHint() && placeholder}
    <div class="help">{placeholder}</div>
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

  .tool.active {
    border-color: rgba(99, 102, 241, 0.65);
    background: rgba(99, 102, 241, 0.16);
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

  .editor-shell {
    width: 100%;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 0.65rem 0.75rem;
    background: var(--bg-surface);
    color: var(--text-primary);
    outline: none;
    overflow: auto;
  }

  .editor {
    min-height: 100%;
  }

  /* TipTap attaches ProseMirror node inside. */
  .editor :global(.ProseMirror) {
    outline: none;
    color: var(--text-primary);
    font-size: 0.95rem;
    line-height: 1.6;
  }

  .editor :global(.ProseMirror p) {
    margin: 0.65rem 0;
  }

  .editor :global(.ProseMirror p:first-child) {
    margin-top: 0;
  }

  .editor-shell:focus-within {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .editor :global(.ProseMirror a) {
    color: var(--color-primary);
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .editor :global(.ProseMirror blockquote) {
    margin: 0.85rem 0;
    padding: 0.65rem 0.8rem;
    border-left: 3px solid rgba(99, 102, 241, 0.55);
    background: rgba(99, 102, 241, 0.08);
    border-radius: 12px;
  }

  .editor :global(.ProseMirror pre) {
    margin: 0.85rem 0;
    padding: 0.85rem 0.95rem;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(0, 0, 0, 0.35);
    overflow: auto;
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  :global([data-theme='light']) .editor :global(.ProseMirror pre) {
    border-color: rgba(0, 0, 0, 0.1);
    background: rgba(0, 0, 0, 0.06);
  }

  /* Placeholder */
  .editor :global(.ProseMirror .is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    float: left;
    color: rgba(255, 255, 255, 0.45);
    pointer-events: none;
    height: 0;
  }

  :global([data-theme='light']) .editor :global(.ProseMirror .is-editor-empty:first-child::before) {
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

<script lang="ts">
  import Icon from './Icon.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { downloadFile } from '$lib/utils/download';

  import { renderAsync } from 'docx-preview';
  import * as XLSX from 'xlsx';

  type AnyFile = {
    id: string;
    original_name: string;
    content_type: string;
    size: number;
    created_at: string;
  };

  let { file, src, downloadUrl } = $props<{
    file: AnyFile;
    src: string;
    downloadUrl: string;
  }>();

  let loading = $state(true);
  let error = $state<string | null>(null);
  let host = $state<HTMLDivElement | null>(null);

  const ext = $derived((file?.original_name || '').split('.').pop()?.toLowerCase() || '');

  const isDocx = $derived(
    ext === 'docx' ||
      file?.content_type ===
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  );
  const isXlsx = $derived(
    ext === 'xlsx' ||
      file?.content_type === 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  );
  const isPptx = $derived(
    ext === 'pptx' ||
      file?.content_type ===
        'application/vnd.openxmlformats-officedocument.presentationml.presentation',
  );

  $effect(() => {
    void file?.id; // track file change
    void render();
  });

  async function render() {
    loading = true;
    error = null;
    if (host) host.innerHTML = '';

    try {
      if (!src) throw new Error('Missing src');
      const res = await fetch(src);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buf = await res.arrayBuffer();

      if (!host) return;

      if (isDocx) {
        // Render DOCX to HTML
        await renderAsync(buf, host, host, {
          className: 'docx',
          ignoreWidth: false,
          ignoreHeight: false,
          ignoreFonts: false,
          breakPages: true,
          useBase64URL: true,
        } as any);
      } else if (isXlsx) {
        // Render first sheet to HTML
        const wb = XLSX.read(buf, { type: 'array' });
        const name = wb.SheetNames?.[0];
        const ws = name ? wb.Sheets[name] : null;
        if (!ws) throw new Error('No sheets');
        const html = XLSX.utils.sheet_to_html(ws, { id: 'sheet' });
        // sheet_to_html returns a full HTML doc; keep only body contents
        const body = html.split('<body>')[1]?.split('</body>')[0] ?? html;
        host.innerHTML = body;
      } else if (isPptx) {
        throw new Error('PPTX preview not supported yet');
      } else {
        throw new Error('Office preview not supported');
      }
    } catch (e: any) {
      error = e?.message || String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="office-shell">
  <div class="office-toolbar">
    <div class="left">
      <Icon name="file-text" size={16} />
      <span class="label">
        {isDocx ? 'DOCX' : isXlsx ? 'XLSX' : isPptx ? 'PPTX' : 'FILE'}
      </span>
    </div>
    <div class="right">
      <button
        class="btn primary"
        type="button"
        onclick={() => downloadFile(downloadUrl, file.original_name)}
      >
        <Icon name="download" size={16} />
        {$t('common.download') || 'Download'}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="center">
      <div class="spinner"></div>
      <div class="hint">{$t('common.loading') || 'Loading...'}</div>
    </div>
  {:else if error}
    <div class="center">
      <Icon name="alert-circle" size={20} />
      <div class="hint">
        {$t('components.lightbox.preview_unavailable') || 'Preview not available'}
        <span class="muted">({error})</span>
      </div>
      <button
        class="btn primary"
        type="button"
        onclick={() => downloadFile(downloadUrl, file.original_name)}
      >
        <Icon name="download" size={16} />
        {$t('components.lightbox.download_file') || 'Download File'}
      </button>
    </div>
  {:else}
    <div class="office-view" bind:this={host}></div>
  {/if}
</div>

<style>
  .office-shell {
    width: 85vw;
    height: 80vh;
    border-radius: 12px;
    overflow: hidden;
    background: white;
    border: 1px solid rgba(255, 255, 255, 0.12);
    display: grid;
    grid-template-rows: auto 1fr;
  }

  .office-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.75rem 0.85rem;
    background: #0b1220;
    color: rgba(255, 255, 255, 0.9);
  }

  .left {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 900;
    letter-spacing: 0.02em;
  }

  .label {
    font-size: 0.9rem;
  }

  .right {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.9);
    padding: 0.4rem 0.6rem;
    cursor: pointer;
    font-weight: 800;
  }

  .btn.primary {
    background: rgba(99, 102, 241, 0.25);
    border-color: rgba(99, 102, 241, 0.4);
  }

  .office-view {
    overflow: auto;
    padding: 1rem;
    color: #111827;
  }

  .center {
    display: grid;
    place-items: center;
    gap: 0.65rem;
    padding: 2.5rem 1rem;
    background: #0b1220;
    color: rgba(255, 255, 255, 0.9);
    text-align: center;
  }

  .hint {
    font-weight: 800;
  }

  .muted {
    color: rgba(255, 255, 255, 0.65);
    font-weight: 700;
  }

  .spinner {
    width: 30px;
    height: 30px;
    border: 3px solid rgba(255, 255, 255, 0.16);
    border-top-color: rgba(99, 102, 241, 0.9);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* XLSX HTML normalizer (generated markup varies) */
  :global(#sheet table) {
    border-collapse: collapse;
    width: 100%;
    font-size: 0.9rem;
  }

  :global(#sheet td),
  :global(#sheet th) {
    border: 1px solid #e5e7eb;
    padding: 0.35rem 0.5rem;
  }

  :global(#sheet th) {
    background: #f3f4f6;
    font-weight: 800;
  }

  @media (max-width: 900px) {
    .office-shell {
      width: 92vw;
      height: 78vh;
    }
  }
</style>

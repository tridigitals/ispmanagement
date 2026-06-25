<script lang="ts">
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    isDesktopApp = false,
    filePath = '',
    fileName = '',
    fileSizeBytes = 0,
    uploading = false,
    error = '',
    onPick,
    onUpload,
  }: {
    isDesktopApp?: boolean;
    filePath?: string;
    fileName?: string;
    fileSizeBytes?: number;
    uploading?: boolean;
    error?: string;
    onPick: (file?: File) => void | Promise<void>;
    onUpload: () => void | Promise<void>;
  } = $props();

  function onPickFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement | null;
    const file = input?.files?.[0];
    if (file) void onPick(file);
    if (input) input.value = '';
  }
</script>

<section class="mix-step">
  <div class="drop-zone">
    <div class="drop-icon"><Icon name="download" size={28} /></div>
    <div>
      <h2>{$t('mixradius.import_wizard.upload_step.title')}</h2>
      <p>Pilih file backup `.sql` atau `.sql.gz` dari server/app lokal untuk distaging dulu.</p>
    </div>
    {#if isDesktopApp}
      <button class="btn" type="button" onclick={() => onPick()} disabled={uploading}>
        <Icon name="folder" size={16} />
        Pilih file
      </button>
    {:else}
      <label class="btn file-trigger" aria-disabled={uploading ? 'true' : 'false'}>
        <Icon name="folder" size={16} />
        Pilih file
        <input type="file" accept=".sql,.gz,.sql.gz" disabled={uploading} onchange={onPickFile} />
      </label>
    {/if}
  </div>

  {#if filePath || fileName}
    <div class="file-card">
      <Icon name="archive" size={18} />
      <div>
        <strong>{fileName || 'MixRadius backup'}</strong>
        <span>{fileSizeBytes ? `${fileSizeBytes.toLocaleString()} bytes` : 'Size unknown'}</span>
        {#if filePath}
          <code>{filePath}</code>
        {:else}
          <code>{$t('mixradius.import_wizard.upload_step.browser_upload')}</code>
        {/if}
      </div>
    </div>
  {/if}

  {#if error}
    <div class="error-line"><Icon name="alert-triangle" size={16} /> {error}</div>
  {/if}

  <div class="step-actions">
    <button class="btn primary" type="button" onclick={onUpload} disabled={(!filePath && !fileName) || uploading}>
      {uploading ? 'Uploading...' : 'Upload & stage'}
      <Icon name="arrow-right" size={16} />
    </button>
  </div>
</section>

<style>
  .mix-step {
    display: grid;
    gap: 16px;
  }

  .drop-zone,
  .file-card {
    display: flex;
    align-items: center;
    gap: 14px;
    border: 1px solid var(--border-color, rgba(148, 163, 184, 0.22));
    background: var(--bg-card, rgba(15, 23, 42, 0.72));
    border-radius: var(--radius-lg);
    padding: 18px;
  }

  .drop-zone {
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .file-trigger {
    position: relative;
    overflow: hidden;
    cursor: pointer;
  }

  .file-trigger[aria-disabled='true'] {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .file-trigger input {
    display: none;
  }

  .drop-icon {
    width: 54px;
    height: 54px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-lg);
    background: rgba(14, 165, 233, 0.14);
    color: #38bdf8;
  }

  h2,
  p {
    margin: 0;
  }

  p,
  .file-card span,
  code {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .file-card > div {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  code {
    word-break: break-all;
  }

  .error-line {
    color: #fca5a5;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .step-actions {
    display: flex;
    justify-content: flex-end;
  }
</style>

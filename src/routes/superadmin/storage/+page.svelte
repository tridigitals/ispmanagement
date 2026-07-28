<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { loadFileManagerModule } from '$lib/components/ui/fileManagerModule';

  let FileManagerComponent = $state<any>(null);
  let fileManagerLoading = $state(false);

  onMount(() => {
    void ensureFileManagerLoaded();
  });

  async function ensureFileManagerLoaded() {
    if (FileManagerComponent || fileManagerLoading) return;

    fileManagerLoading = true;
    try {
      const { FileManagerComponent: FileManager } = await loadFileManagerModule();
      FileManagerComponent = FileManager;
    } finally {
      fileManagerLoading = false;
    }
  }
</script>

<div class="sa-storage fade-in">
  <div class="page-head">
    <div>
      <div class="crumbs">
        {$t('superadmin.storage.crumbs.root')}
        <span class="crumb-sep">›</span>
        <b>{$t('superadmin.storage.crumbs.storage')}</b>
      </div>
      <h1>{$t('superadmin.storage.title') || 'Storage'}</h1>
    </div>
  </div>

  {#if FileManagerComponent}
    <FileManagerComponent mode="admin" />
  {:else}
    <div class="storage-loader" aria-busy={fileManagerLoading}></div>
  {/if}
</div>

<style>
  .sa-storage {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1280px;
    margin: 0 auto;
    color: var(--text-primary);
  }

  .page-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
    flex-wrap: wrap;
  }

  .crumbs {
    font-size: 0.75rem;
    color: var(--text-secondary);
    opacity: 0.75;
    margin-bottom: 6px;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .crumbs b { font-weight: 500; opacity: 1; }

  .page-head h1 {
    font-size: 1.45rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
  }

  .storage-loader {
    min-height: 20rem;
  }
</style>

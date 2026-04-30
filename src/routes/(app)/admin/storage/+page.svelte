<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { can } from '$lib/stores/auth';
  import { loadFileManagerModule } from '$lib/components/ui/fileManagerModule';

  let FileManagerComponent = $state<any>(null);
  let fileManagerLoading = $state(false);

  onMount(() => {
    if (!$can('read', 'storage_console')) {
      goto('/unauthorized');
      return;
    }

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

{#if FileManagerComponent}
  <FileManagerComponent mode="tenant" showHeader={false} />
{:else}
  <div class="storage-loader" aria-busy={fileManagerLoading}></div>
{/if}

<style>
  .storage-loader {
    min-height: 20rem;
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
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

{#if FileManagerComponent}
  <FileManagerComponent mode="admin" />
{:else}
  <div class="storage-loader" aria-busy={fileManagerLoading}></div>
{/if}

<style>
  .storage-loader {
    min-height: 20rem;
  }
</style>

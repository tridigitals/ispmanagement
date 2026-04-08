<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { can } from '$lib/stores/auth';
  import FileManager from '$lib/components/ui/FileManager.svelte';

  onMount(() => {
    if (!$can('read', 'storage_console')) {
      goto('/unauthorized');
    }
  });
</script>

<FileManager mode="tenant" showHeader={false} />

<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { can } from '$lib/stores/auth';
  import FileManager from '$lib/components/ui/FileManager.svelte';

  onMount(() => {
    if (!$can('read', 'storage') && !$can('upload', 'storage') && !$can('delete', 'storage')) {
      goto('/unauthorized');
    }
  });
</script>

<FileManager mode="admin" showHeader={false} />

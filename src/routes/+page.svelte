<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    // Preserve current URL params (e.g. ?reason=expired, ?invite=...)
    const search = window.location.search;
    goto(`/login${search}`, { replaceState: true });
  });
</script>

<div class="redirect" style="display:flex;align-items:center;justify-content:center;min-height:100vh;color:var(--text-muted)">Redirecting...</div>

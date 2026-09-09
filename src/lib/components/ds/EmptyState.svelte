<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  let {
    title = 'Tidak ada data',
    hint = '',
    icon = 'inbox',
  }: { title?: string; hint?: string; icon?: IconName } = $props();
</script>

<div class="flex flex-col items-center justify-center gap-1.5 px-4 py-8 text-center">
  <span class="text-ink-300"><Icon name={icon} size={24} /></span>
  <div class="text-sm font-medium text-ink-700">{title}</div>
  {#if hint}
    <p class="max-w-sm text-sm text-ink-500">{hint}</p>
  {/if}
</div>

<script lang="ts">
  import {
    getLucideIconModuleLoader,
    type LucideModule,
  } from '$lib/utils/iconModules';
  import { getLucideIconImportPath } from '$lib/utils/iconResolver';

  let {
    name,
    size = 18,
    strokeWidth = 2,
    color = 'currentColor',
    class: className = '',
    ...restProps
  } = $props();

  let IconComponent = $state<any>(null);
  let loadId = 0;

  async function loadIcon(iconName: string | undefined) {
    const nextLoadId = ++loadId;
    const iconPath = getLucideIconImportPath(iconName);
    const loadModule = getLucideIconModuleLoader(iconPath);
    const loadFallback = getLucideIconModuleLoader('help-circle');

    try {
      if (!loadModule) {
        throw new Error(`Unknown lucide icon: ${iconPath}`);
      }
      const module = await loadModule();
      if (nextLoadId === loadId) {
        IconComponent = module.default;
      }
    } catch {
      if (!loadFallback) return;
      const fallback = await loadFallback();
      if (nextLoadId === loadId) {
        IconComponent = fallback.default;
      }
    }
  }

  $effect(() => {
    void loadIcon(name);
  });
</script>

{#if IconComponent}
  <IconComponent {size} {strokeWidth} {color} class={className} {...restProps} />
{:else}
  <span
    class={className}
    style:display="inline-flex"
    style:width={`${size}px`}
    style:height={`${size}px`}
    aria-hidden="true"
  ></span>
{/if}

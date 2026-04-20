<script lang="ts">
  import { getLucideIconImportPath } from '$lib/utils/iconResolver';

  type LucideModule = {
    default: any;
  };

  const iconModules = import.meta.glob<LucideModule>(
    '../../../../node_modules/lucide-svelte/dist/icons/*.js',
  );

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
    const modulePath = `../../../../node_modules/lucide-svelte/dist/icons/${iconPath}.js`;
    const fallbackPath = '../../../../node_modules/lucide-svelte/dist/icons/help-circle.js';

    try {
      const loadModule = iconModules[modulePath];
      if (!loadModule) {
        throw new Error(`Unknown lucide icon: ${iconPath}`);
      }
      const module = await loadModule();
      if (nextLoadId === loadId) {
        IconComponent = module.default;
      }
    } catch {
      const loadFallback = iconModules[fallbackPath];
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

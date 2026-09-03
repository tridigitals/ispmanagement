<!--
  Ds/Icon — ikon SVG sinkron.

  Beda dengan lib/components/ui/Icon.svelte yang memuat tiap ikon lewat dynamic
  import (menyebabkan ikon pop-in dan layout shift), komponen ini menulis path
  langsung sehingga ikon ikut render pertama.
-->
<script lang="ts">
  import { icons, type IconName } from './icons';

  interface Props {
    name: IconName;
    /** Ukuran sisi dalam px. Default 16 supaya sejajar dengan teks 13px. */
    size?: number;
    strokeWidth?: number;
    class?: string;
    /** Isi kalau ikon berdiri sendiri tanpa teks pendamping. */
    label?: string;
  }

  let { name, size = 16, strokeWidth = 1.75, class: cls = '', label }: Props = $props();

  const path = $derived(icons[name] ?? icons.grid);
</script>

<svg
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class={cls}
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  focusable="false"
>
  {#if label}<title>{label}</title>{/if}
  <path d={path} />
</svg>

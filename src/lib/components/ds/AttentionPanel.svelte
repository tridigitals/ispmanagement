<!--
  Ds/AttentionPanel — daftar hal yang perlu tindakan.

  Ini pengganti baris stat card di dashboard. Prinsipnya: dashboard harus
  menampilkan pekerjaan, bukan angka yang tidak bisa ditindaklanjuti.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  export interface AttentionItem {
    icon: IconName;
    /** Masalahnya apa. */
    title: string;
    /** Angka/konteks pendukung. */
    detail: string;
    /** Teks aksi, contoh 'Tinjau 473 invoice'. */
    action: string;
    href?: string;
    severity?: 'high' | 'medium' | 'low';
  }

  interface Props {
    items: AttentionItem[];
    title?: string;
  }

  let { items, title = 'Perlu tindakan' }: Props = $props();

  const dot = { high: 'bg-red-500', medium: 'bg-amber-500', low: 'bg-sky-500' };
</script>

<section class="overflow-hidden rounded-xl bg-amber-50/40 ring-1 ring-inset ring-amber-200">
  <div class="flex h-12 items-center gap-2 border-b border-amber-200/70 px-5">
    <Icon name="alert" size={15} class="text-amber-700" />
    <h2 class="text-base font-semibold text-amber-900">{title}</h2>
    <span class="num ml-auto text-sm text-amber-800">{items.length} item</span>
  </div>

  <ul class="divide-y divide-amber-200/50">
    {#each items as it}
      <li class="flex items-center gap-4 px-5 py-3">
        <span class="size-1.5 shrink-0 rounded-full {dot[it.severity ?? 'medium']}"></span>
        <Icon name={it.icon} size={15} class="shrink-0 text-amber-700" />
        <div class="min-w-0 flex-1">
          <div class="truncate text-base font-medium text-ink-900">{it.title}</div>
          <div class="truncate text-sm text-ink-500">{it.detail}</div>
        </div>
        <a
          href={it.href ?? '#'}
          class="focus-ring inline-flex h-7 shrink-0 items-center gap-1 rounded-md bg-white px-2.5 text-sm font-medium text-amber-800 ring-1 ring-inset ring-amber-300 hover:bg-amber-50"
        >
          {it.action}
          <Icon name="chevronRight" size={13} />
        </a>
      </li>
    {/each}
  </ul>
</section>

<!--
  Ds/Badge — badge status dengan kontras terjamin.

  Warna tidak boleh dilewatkan pemanggil: cukup kirim status apa adanya dari
  API, pemetaan tone dilakukan di tokens.ts. Ini mencegah kembalinya 131 warna
  hex unik yang tersebar sebelum redesign.
-->
<script lang="ts">
  import { badgeClass, toneOf, type StatusTone } from './tokens';

  interface Props {
    /** Nilai status mentah dari API, contoh 'suspended' atau 'paid'. */
    status?: string | null;
    /** Paksa tone tertentu kalau nilainya bukan status domain. */
    tone?: StatusTone;
    /** Label tampil. Default: status apa adanya. */
    label?: string;
    mode?: 'light' | 'dark';
    class?: string;
  }

  let { status = null, tone, label, mode = 'light', class: cls = '' }: Props = $props();

  const resolvedTone = $derived(tone ?? toneOf(status));
  const text = $derived(label ?? (status ? String(status).replace(/_/g, ' ') : ''));
</script>

<span
  class="inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-medium whitespace-nowrap ring-1 ring-inset {badgeClass(
    resolvedTone,
    mode,
  )} {cls}"
>
  {text}
</span>

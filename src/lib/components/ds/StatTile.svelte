<!--
  Ds/StatTile — satu angka + konteks.

  Aturan: setiap tile WAJIB punya `hint` yang menjelaskan angkanya (basis
  perbandingan, jumlah, atau persentase). Stat card lama hanya menampilkan
  "Total 548" tanpa konteks sehingga tidak bisa dipakai mengambil keputusan.
-->
<script lang="ts">
  interface Props {
    label: string;
    value: string;
    /** Konteks wajib: "473 invoice · 97,9%", "dari 540 langganan", dst. */
    hint: string;
    tone?: 'neutral' | 'positive' | 'negative' | 'warning';
    mode?: 'light' | 'dark';
  }

  let { label, value, hint, tone = 'neutral', mode = 'light' }: Props = $props();
  const dark = $derived(mode === 'dark');

  const lightTone = {
    neutral: 'text-ink-900',
    positive: 'text-emerald-700',
    negative: 'text-red-700',
    warning: 'text-amber-800',
  };
  const darkTone = {
    neutral: 'text-slate-100',
    positive: 'text-emerald-300',
    negative: 'text-red-300',
    warning: 'text-amber-300',
  };
</script>

<div class="min-w-0">
  <div
    class="mb-1 text-xs font-medium tracking-wide uppercase {dark ? 'text-slate-400' : 'text-ink-400'}"
  >
    {label}
  </div>
  <div class="num text-lg leading-none font-semibold {(dark ? darkTone : lightTone)[tone]}">
    {value}
  </div>
  <div class="mt-1.5 text-xs {dark ? 'text-slate-400' : 'text-ink-500'}">{hint}</div>
</div>

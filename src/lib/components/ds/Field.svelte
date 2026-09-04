<!--
  Ds/Field — satu baris form untuk seluruh aplikasi.

  Halaman pengaturan lama menulis markup label+input sendiri di 5 file berbeda
  (+page.svelte 2.098 baris, SettingsEmailTab 746, SettingsPaymentTab 859,
  SettingsServiceTab 1.158, SettingsCompanyTab 535). Akibatnya:
    - Label kadang <label for> kadang <span>, jadi sebagian input tidak bisa
      difokuskan dengan mengklik labelnya dan tidak terbaca screen reader.
    - Teks bantuan kadang <small>, kadang <p class="hint">, kadang tidak ada.
    - Toggle boolean dibuat 3 cara berbeda: checkbox 16px (gagal WCAG 2.5.8),
      <select> "true"/"false", dan tombol kustom.

  Satu komponen ini menggantikan semuanya. Setiap tipe input tetap satu elemen
  form asli (bukan div yang dipaksa mirip input), jadi keyboard, autofill, dan
  validasi bawaan browser tetap jalan.

  Nilai SELALU string. Backend menyimpan settings sebagai string, jadi konversi
  dilakukan di tepi (renderer) bukan disebar ke pemanggil — itu sumber bug
  "true" vs true di halaman lama.
-->
<script lang="ts">
  import Icon from './Icon.svelte';

  export type FieldType = 'text' | 'number' | 'password' | 'email' | 'textarea' | 'select' | 'toggle';

  export interface FieldOption {
    value: string;
    label: string;
  }

  interface Props {
    id: string;
    label: string;
    value: string;
    type?: FieldType;
    /** Teks bantuan di bawah input. Pakai untuk menjelaskan efeknya, bukan mengulang label. */
    help?: string;
    /** Pesan galat; menimpa help dan menandai input invalid. */
    error?: string | null;
    options?: FieldOption[];
    placeholder?: string;
    /** Satuan yang ditempel di kanan input angka, misal 'menit', 'ms', 'hari'. */
    suffix?: string;
    min?: number;
    max?: number;
    rows?: number;
    disabled?: boolean;
    /** Tandai baris yang nilainya berbeda dari tersimpan. */
    dirty?: boolean;
    /**
     * Label di ATAS input (bukan kiri). Default false = layout 15rem kiri
     * yang dirancang untuk form halaman penuh; di dalam modal 720px layout
     * itu menyempitkan input grid 2-kolom sampai 69px (terukur, modal
     * sunting pengumuman).
     */
    stacked?: boolean;
    onchange: (value: string) => void;
  }

  let {
    id,
    label,
    value,
    type = 'text',
    help,
    error = null,
    options = [],
    placeholder,
    suffix,
    min,
    max,
    rows = 3,
    disabled = false,
    dirty = false,
    stacked = false,
    onchange,
  }: Props = $props();

  const on = $derived(value === 'true');
  const describedBy = $derived(help || error ? `${id}-desc` : undefined);

  const inputBase =
    'w-full rounded-lg bg-white px-3 text-base text-ink-900 ring-1 ring-inset transition-colors ' +
    'placeholder:text-ink-400 focus:outline-2 focus:outline-offset-0 focus:outline-brand-600 ' +
    'disabled:bg-ink-50 disabled:text-ink-400';
  const ring = $derived(error ? 'ring-red-400' : 'ring-ink-200 hover:ring-ink-300');
</script>

<div
  class="grid gap-1.5 py-3 {stacked
    ? ''
    : 'sm:grid-cols-[minmax(0,15rem)_minmax(0,1fr)] sm:gap-x-6 sm:py-3.5'}"
>
  <div class="flex min-w-0 items-start gap-1.5 {stacked ? '' : 'sm:pt-1.5'}">
    <label for={id} class="text-base font-medium text-ink-800">{label}</label>
    {#if dirty}
      <!-- Titik kecil, bukan badge: menandai baris berubah tanpa menarik perhatian
           lebih dari isinya sendiri. -->
      <span
        class="mt-1.5 size-1.5 shrink-0 rounded-full bg-amber-500"
        title="Belum disimpan"
        aria-label="Belum disimpan"
      ></span>
    {/if}
  </div>

  <div class="min-w-0">
    {#if type === 'toggle'}
      <button
        {id}
        type="button"
        role="switch"
        aria-checked={on}
        aria-describedby={describedBy}
        {disabled}
        onclick={() => onchange(on ? 'false' : 'true')}
        class="focus-ring inline-flex h-6 w-11 shrink-0 items-center rounded-full p-0.5 transition-colors
          {on ? 'bg-brand-600' : 'bg-ink-300'} disabled:opacity-50"
      >
        <span
          class="size-5 rounded-full bg-white shadow-sm transition-transform {on
            ? 'translate-x-5'
            : 'translate-x-0'}"
        ></span>
      </button>
    {:else if type === 'select'}
      <div class="relative">
        <select
          {id}
          {disabled}
          aria-describedby={describedBy}
          aria-invalid={error ? 'true' : undefined}
          value={value}
          onchange={(e) => onchange((e.currentTarget as HTMLSelectElement).value)}
          class="{inputBase} {ring} h-9 appearance-none pr-9"
        >
          {#each options as opt (opt.value)}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
        <Icon
          name="chevronDown"
          size={15}
          class="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-ink-400"
        />
      </div>
    {:else if type === 'textarea'}
      <textarea
        {id}
        {rows}
        {placeholder}
        {disabled}
        aria-describedby={describedBy}
        aria-invalid={error ? 'true' : undefined}
        value={value}
        oninput={(e) => onchange((e.currentTarget as HTMLTextAreaElement).value)}
        class="{inputBase} {ring} resize-y py-2 leading-relaxed"
      ></textarea>
    {:else}
      <div class="flex items-center gap-2">
        <input
          {id}
          {type}
          {placeholder}
          {disabled}
          {min}
          {max}
          aria-describedby={describedBy}
          aria-invalid={error ? 'true' : undefined}
          value={value}
          oninput={(e) => onchange((e.currentTarget as HTMLInputElement).value)}
          class="{inputBase} {ring} h-9 {type === 'number' ? 'num max-w-40' : ''}"
        />
        {#if suffix}
          <span class="shrink-0 text-base text-ink-500">{suffix}</span>
        {/if}
      </div>
    {/if}

    {#if error}
      <p id={describedBy} class="mt-1.5 text-sm text-red-700">{error}</p>
    {:else if help}
      <p id={describedBy} class="mt-1.5 text-sm text-ink-500">{help}</p>
    {/if}
  </div>
</div>

<!--
  Ds/Button — satu definisi tombol untuk seluruh aplikasi.

  Sebelum redesign, kelas `.btn` didefinisikan ulang di 64 file berbeda dan
  varian primary-nya memakai #8b9cff dengan teks putih (rasio kontras 2,53:1,
  GAGAL WCAG AA). Varian di sini sudah diverifikasi:
    primary   putih di atas ink-900  = 17,72:1
    warning   amber-800 di atas amber-50 = 6,84:1
    dprimary  night-0 di atas sky-400 = 9,42:1

  Aturan pemakaian: maksimal SATU tombol `primary` per layar.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';

  type Variant =
    | 'primary'
    | 'secondary'
    | 'ghost'
    | 'warning'
    | 'danger'
    | 'dprimary'
    | 'dsecondary'
    | 'dghost';

  interface Props {
    variant?: Variant;
    size?: 'sm' | 'md';
    icon?: IconName;
    /** Wajib diisi kalau tombol hanya berisi ikon. */
    label?: string;
    href?: string;
    type?: 'button' | 'submit' | 'reset';
    disabled?: boolean;
    loading?: boolean;
    class?: string;
    onclick?: (event: MouseEvent) => void;
    children?: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'md',
    icon,
    label,
    href,
    type = 'button',
    disabled = false,
    loading = false,
    class: cls = '',
    onclick,
    children,
  }: Props = $props();

  const variants: Record<Variant, string> = {
    primary: 'bg-ink-900 text-white hover:bg-ink-700 focus-visible:outline-ink-900',
    secondary:
      'bg-white text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50 focus-visible:outline-brand-600',
    ghost: 'text-ink-500 hover:bg-ink-100 hover:text-ink-900 focus-visible:outline-brand-600',
    warning:
      'bg-amber-50/70 text-amber-800 ring-1 ring-inset ring-amber-300 hover:bg-amber-50 focus-visible:outline-amber-600',
    danger:
      'bg-red-50 text-red-700 ring-1 ring-inset ring-red-300 hover:bg-red-100 focus-visible:outline-red-600',
    dprimary:
      'bg-sky-400 text-night-0 font-semibold hover:bg-sky-300 focus-visible:outline-sky-400',
    dsecondary:
      'bg-white/5 text-slate-200 ring-1 ring-inset ring-white/10 hover:bg-white/10 focus-visible:outline-sky-400',
    dghost: 'text-slate-400 hover:bg-white/5 hover:text-slate-200 focus-visible:outline-sky-400',
  };

  const sizes = {
    sm: 'h-7 px-2.5 text-sm gap-1',
    md: 'h-9 px-3 text-base gap-1.5',
  };

  const base =
    'inline-flex items-center justify-center rounded-lg font-medium whitespace-nowrap transition-colors ' +
    'focus-visible:outline-2 focus-visible:outline-offset-2 disabled:opacity-50 disabled:pointer-events-none';

  const classes = $derived(`${base} ${sizes[size]} ${variants[variant]} ${cls}`);
  const iconSize = $derived(size === 'sm' ? 14 : 15);
</script>

{#if href}
  <a {href} class={classes} aria-label={label && !children ? label : undefined}>
    {#if icon}<Icon name={icon} size={iconSize} />{/if}
    {#if children}{@render children()}{:else if label}{label}{/if}
  </a>
{:else}
  <button
    {type}
    {onclick}
    disabled={disabled || loading}
    class={classes}
    aria-label={label && !children ? label : undefined}
    aria-busy={loading ? 'true' : undefined}
  >
    {#if loading}
      <Icon name="refresh" size={iconSize} class="animate-spin" />
    {:else if icon}
      <Icon name={icon} size={iconSize} />
    {/if}
    {#if children}{@render children()}{:else if label}{label}{/if}
  </button>
{/if}

<!--
  PageSkeleton — pengganti spinner global "+layout" dan teks "Memuat…" layout v2.

  Kenapa bukan spinner polos: transisi antar-halaman v2 terasa "mati" (kanvas
  hitam + spinner kecil). Komponen ini mempre-render siluet shell (rail, topbar,
  konten) dengan shimmer sehingga perpindahan rute terasa berkelanjutan.
  Prop `dark` = siluet berkanvas gelap untuk rute tema gelap (pay, install,
  superadmin, wallboard, legacy) supaya loader tidak berkedip putih.
  Murni CSS animation, pointer-events-none, tidak menyentuh halaman di bawahnya.
-->
<script lang="ts">
  import { t } from 'svelte-i18n';

  let {
    label = '',
    rows = 4,
    dark = false,
  }: { label?: string; rows?: number; dark?: boolean } = $props();
  const tr = $derived(label ? label : $t('common.loading'));
</script>

<div
  class="skel pointer-events-none fixed inset-0 z-[60] select-none"
  class:skel-dark={dark}
  role="status"
  aria-live="polite"
  aria-label={tr}
>
  <div class="skel-canvas absolute inset-0 flex">
    <!-- Rail siluet -->
    <div class="skel-rail hidden w-[72px] shrink-0 flex-col items-center gap-4 py-4 lg:flex">
      <div class="shimmer size-9 rounded-xl"></div>
      {#each Array(7) as _}
        <div class="shimmer size-8 rounded-lg"></div>
      {/each}
    </div>

    <div class="flex min-w-0 flex-1 flex-col">
      <!-- Topbar siluet -->
      <div class="skel-topbar flex h-[52px] shrink-0 items-center gap-3 px-5">
        <div class="shimmer h-4 w-40 rounded-md"></div>
        <div class="ml-auto flex items-center gap-3">
          <div class="shimmer size-8 rounded-lg"></div>
          <div class="shimmer size-8 rounded-full"></div>
        </div>
      </div>

      <!-- Konten siluet -->
      <div class="flex-1 overflow-hidden p-5 lg:p-7">
        <div class="mx-auto max-w-[1400px]">
          <div class="shimmer mb-2 h-6 w-56 rounded-md"></div>
          <div class="shimmer mb-6 h-3.5 w-80 rounded"></div>
          <div class="mb-6 grid grid-cols-2 gap-4 sm:grid-cols-4">
            {#each Array(4) as _}
              <div class="shimmer skel-box h-[86px] rounded-xl"></div>
            {/each}
          </div>
          <div class="skel-box overflow-hidden rounded-xl">
            <div class="skel-line-top shimmer h-9 opacity-60"></div>
            {#each Array(rows) as _}
              <div class="skel-line flex items-center gap-4 px-4 py-3.5">
                <div class="shimmer h-3.5 w-1/4 rounded"></div>
                <div class="shimmer h-3.5 w-1/6 rounded"></div>
                <div class="shimmer h-3.5 w-1/6 rounded"></div>
                <div class="shimmer ml-auto h-5 w-16 rounded-full"></div>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Bar progres tak-tentu di puncak -->
  <div class="indeterminate-track"><div class="indeterminate-bar"></div></div>
</div>

<style>
  .shimmer {
    border-radius: 8px;
    background: linear-gradient(90deg, var(--color-ink-100) 25%, var(--color-ink-50, #fafafa) 50%, var(--color-ink-100) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }

  .skel-canvas {
    background: var(--color-ink-50, #fafafa);
  }
  .skel-rail {
    background: var(--color-ink-0, #fff);
    border-right: 1px solid var(--color-ink-100);
  }
  .skel-topbar {
    background: var(--color-ink-0, #fff);
    border-bottom: 1px solid var(--color-ink-100);
  }
  .skel-box {
    background: var(--color-ink-0, #fff);
    border: 1px solid var(--color-ink-100);
  }
  .skel-line-top {
    border-bottom: 1px solid var(--color-ink-100);
  }
  .skel-line {
    border-bottom: 1px solid var(--color-ink-100);
  }
  .skel-line:last-child {
    border-bottom: 0;
  }

  /* --- varian gelap: pay/install/superadmin/wallboard/legacy --- */
  .skel-dark .skel-canvas {
    background: #08090d;
  }
  .skel-dark .skel-rail,
  .skel-dark .skel-topbar,
  .skel-dark .skel-box {
    background: #11141c;
  }
  .skel-dark .skel-rail,
  .skel-dark .skel-topbar,
  .skel-dark .skel-box,
  .skel-dark .skel-line-top,
  .skel-dark .skel-line {
    border-color: rgba(148, 163, 184, 0.14);
  }
  .skel-dark .shimmer {
    background: linear-gradient(90deg, #171b25 25%, #202638 50%, #171b25 75%);
    background-size: 200% 100%;
  }

  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .indeterminate-track {
    position: absolute;
    inset: 0 auto auto 0;
    height: 2px;
    width: 100%;
    overflow: hidden;
    background: transparent;
  }

  .indeterminate-bar {
    height: 100%;
    width: 34%;
    border-radius: 9999px;
    background: linear-gradient(90deg, var(--color-brand-500), var(--color-brand-700));
    animation: slide 1.1s ease-in-out infinite;
  }

  @keyframes slide {
    0% {
      transform: translateX(-110%);
    }
    100% {
      transform: translateX(340%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .shimmer,
    .indeterminate-bar {
      animation: none;
    }
  }
</style>

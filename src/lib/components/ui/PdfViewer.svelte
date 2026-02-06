<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { downloadFile } from '$lib/utils/download';
  import { isTauri } from '@tauri-apps/api/core';

  // pdfjs-dist legacy build: better compatibility with older WebView2 runtimes.
  import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist/legacy/build/pdf.mjs';
  import workerSrc from 'pdfjs-dist/legacy/build/pdf.worker.min.mjs?url';

  GlobalWorkerOptions.workerSrc = workerSrc;

  const DEV = import.meta.env.DEV;
  function dlog(...args: any[]) {
    if (DEV) console.debug('[PdfViewer]', ...args);
  }

  let { src, downloadUrl, filename, showToolbar = true } = $props<{
    src: string;
    downloadUrl: string;
    filename: string;
    showToolbar?: boolean;
  }>();

  const fetchUrl = $derived(downloadUrl || src);

  let container = $state<HTMLDivElement | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let scale = $state(1.25);

  let pdfDoc: any = null;
  let pdfData: ArrayBuffer | null = null;
  let renderToken = 0;
  let lastSrc = $state<string | null>(null);
  let stage = $state<'idle' | 'fetch' | 'parse' | 'render'>('idle');
  let blankWarning = $state<string | null>(null);

  function assetUrl(path: string) {
    // This component is client-only in practice, but keep SSR-safe module init.
    if (typeof window === 'undefined') return path;
    return new URL(path, window.location.href).toString();
  }

  const PDFJS_ASSETS = {
    cMapUrl: assetUrl('/pdfjs/cmaps/'),
    standardFontDataUrl: assetUrl('/pdfjs/standard_fonts/'),
    wasmUrl: assetUrl('/pdfjs/wasm/'),
  };

  // Wait until the container is mounted; otherwise the first render may no-op and never re-run.
  $effect(() => {
    container;
    fetchUrl;
    scale;

    if (!container || !fetchUrl) return;

    if (fetchUrl !== lastSrc) {
      lastSrc = fetchUrl;
      pdfData = null; // refetch for new file
    }

    void loadAndRender(fetchUrl);
  });

  async function loadAndRender(url: string) {
    const token = ++renderToken;
    loading = true;
    error = null;
    stage = 'idle';
    blankWarning = null;

    try {
      if (!url) throw new Error('Missing PDF url');

      // Fetch once, re-render from memory on zoom changes.
      if (!pdfData) {
        stage = 'fetch';
        // Use the download endpoint for fetching bytes; some WebViews behave oddly with inline
        // streaming + range responses.
        const ac = new AbortController();
        const fetchPromise = fetch(url, { signal: ac.signal });
        const timeoutPromise = new Promise<never>((_, reject) =>
          setTimeout(() => {
            try {
              ac.abort();
            } catch {
              // ignore
            }
            reject(new Error('PDF fetch timeout'));
          }, 20_000),
        );

        const res = (await Promise.race([fetchPromise, timeoutPromise])) as Response;
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        dlog('fetch ok', {
          url,
          status: res.status,
          contentType: res.headers.get('content-type'),
          contentLength: res.headers.get('content-length'),
          disposition: res.headers.get('content-disposition'),
        });
        pdfData = (await Promise.race([
          res.arrayBuffer(),
          new Promise<never>((_, reject) =>
            setTimeout(() => reject(new Error('PDF read timeout')), 20_000),
          ),
        ])) as ArrayBuffer;

        const u8 = new Uint8Array(pdfData);
        const sig = String.fromCharCode(...u8.slice(0, 5));
        dlog('pdf bytes', { length: u8.byteLength, signature: sig });
        if (sig !== '%PDF-') {
          dlog('unexpected signature; first 32 bytes', Array.from(u8.slice(0, 32)));
        }
      }

      if (token !== renderToken) return;

      // (Re)load pdf document for this render.
      stage = 'parse';
      // In desktop WebViews (esp. WebView2), PDF.js can hang if it tries to lazy-load
      // cmaps/standard_fonts. We ship those assets into `static/pdfjs/` to keep it deterministic.
      dlog('pdfjs assets', PDFJS_ASSETS);
      if (DEV) {
        try {
          const ac = new AbortController();
          setTimeout(() => ac.abort(), 4000);
          const r = await fetch(`${PDFJS_ASSETS.wasmUrl}openjpeg.wasm`, { signal: ac.signal });
          dlog('openjpeg.wasm probe', { ok: r.ok, status: r.status });
        } catch (e) {
          dlog('openjpeg.wasm probe failed', e);
        }
      }
      const task = getDocument({
        data: pdfData!,
        disableRange: true,
        disableStream: true,
        cMapUrl: PDFJS_ASSETS.cMapUrl,
        cMapPacked: true,
        standardFontDataUrl: PDFJS_ASSETS.standardFontDataUrl,
        wasmUrl: PDFJS_ASSETS.wasmUrl,
        // Avoid a known flakiness in embedded WebViews where the fake-worker path
        // fails to propagate factory base-urls to the evaluator.
        useWorkerFetch: false,
        useSystemFonts: true,
        disableFontFace: true,
        isEvalSupported: false,
        enableXfa: true,
        // Some WebView2 setups struggle to load module workers from dev-server URLs.
        // Falling back to main-thread parsing fixes the "infinite loading" symptom.
        disableWorker: isTauri(),
      } as any);
      pdfDoc = await Promise.race([
        task.promise,
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error('PDF parse timeout')), 20_000),
        ),
      ]);
      dlog('parsed', { pages: pdfDoc?.numPages, fingerprint: pdfDoc?.fingerprints?.[0] });

      if (token !== renderToken) return;
      stage = 'render';
      await renderAllPages(pdfDoc, scale, token);
    } catch (e: any) {
      // eslint-disable-next-line no-console
      console.error('[PdfViewer] failed', e);
      error = e?.message || String(e);
    } finally {
      if (token === renderToken) loading = false;
    }
  }

  async function renderAllPages(doc: any, scaleValue: number, token: number) {
    if (!container) return;
    container.innerHTML = '';

    const total: number = doc.numPages || 0;
    let sawInk = false;

    for (let i = 1; i <= total; i++) {
      if (token !== renderToken) return;

      const page = await Promise.race([
        doc.getPage(i),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`PDF getPage(${i}) timeout`)), 10_000),
        ),
      ]);
      const viewport = page.getViewport({ scale: scaleValue });
      dlog('render page', { i, width: viewport.width, height: viewport.height });

      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');

      // HiDPI: keep content crisp, and avoid some WebView2 scaling quirks.
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.floor(viewport.width * dpr);
      canvas.height = Math.floor(viewport.height * dpr);
      canvas.className = 'pdf-canvas';
      canvas.style.width = `${Math.floor(viewport.width)}px`;
      canvas.style.height = `${Math.floor(viewport.height)}px`;

      container.appendChild(canvas);

      const transform = dpr !== 1 ? ([dpr, 0, 0, dpr, 0, 0] as const) : undefined;
      const renderTask = page.render({ canvasContext: ctx, viewport, transform });
      await Promise.race([
        renderTask.promise,
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`PDF render page ${i} timeout`)), 20_000),
        ),
      ]);

      // Detect "blank white" renders (common when image decoders/fonts can't be resolved).
      // Use a small grid of samples and only warn if we see *no* "ink" across all pages.
      try {
        const xs = [0.25, 0.5, 0.75];
        const ys = [0.25, 0.5, 0.75];
        for (const xf of xs) {
          for (const yf of ys) {
            if (sawInk) break;
            const x = Math.min(canvas.width - 3, Math.max(0, Math.floor(canvas.width * xf)));
            const y = Math.min(canvas.height - 3, Math.max(0, Math.floor(canvas.height * yf)));
            const data = ctx.getImageData(x, y, 3, 3).data;

            // Compute average luminance; if any sample is "dark enough", consider it ink.
            for (let p = 0; p < data.length; p += 4) {
              const r = data[p] ?? 255;
              const g = data[p + 1] ?? 255;
              const b = data[p + 2] ?? 255;
              const a = data[p + 3] ?? 255;
              if (a < 8) continue;
              const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
              if (lum < 240) {
                sawInk = true;
                break;
              }
            }
          }
        }
      } catch (e) {
        dlog('pixel sample failed', e);
      }
    }

    if (!sawInk) {
      blankWarning =
        get(t)('components.lightbox.pdf_blank_warning') ||
        'Preview looks blank. Try download/open file.';
      dlog('blank warning triggered', { pages: total });
    }
  }

  function zoomIn() {
    scale = Math.min(3, +(scale + 0.25).toFixed(2));
    if (fetchUrl) void loadAndRender(fetchUrl);
  }

  function zoomOut() {
    scale = Math.max(0.75, +(scale - 0.25).toFixed(2));
    if (fetchUrl) void loadAndRender(fetchUrl);
  }

  function resetZoom() {
    scale = 1.25;
    if (fetchUrl) void loadAndRender(fetchUrl);
  }

  function handleKeydown(e: KeyboardEvent) {
    // Provide zoom shortcuts even when the toolbar is hidden (Lightbox controls navigation).
    if ((e.ctrlKey || e.metaKey) && (e.key === '+' || e.key === '=')) {
      e.preventDefault();
      zoomIn();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === '-') {
      e.preventDefault();
      zoomOut();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === '0') {
      e.preventDefault();
      resetZoom();
    }
  }

  onDestroy(() => {
    renderToken++;
    pdfDoc = null;
    pdfData = null;
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="pdf-shell">
  {#if showToolbar}
    <div class="pdf-toolbar">
      <div class="left">
        <Icon name="file-text" size={16} />
        <span class="label">PDF</span>
        <span class="sep"></span>
        <span class="zoom">{Math.round(scale * 100)}%</span>
      </div>

      <div class="right">
        <button
          class="btn"
          type="button"
          onclick={zoomOut}
          title={$t('common.previous') || 'Zoom out'}
        >
          <Icon name="minus" size={16} />
        </button>
        <button class="btn" type="button" onclick={zoomIn} title={$t('common.next') || 'Zoom in'}>
          <Icon name="plus" size={16} />
        </button>
        <button
          class="btn"
          type="button"
          onclick={resetZoom}
          title={$t('common.reset') || 'Reset'}
        >
          <Icon name="refresh-cw" size={16} />
        </button>
        <button
          class="btn primary"
          type="button"
          onclick={() => downloadFile(downloadUrl, filename)}
        >
          <Icon name="download" size={16} />
          {$t('common.download') || 'Download'}
        </button>
      </div>
    </div>
  {:else}
    <div class="pdf-toolbar hidden" aria-hidden="true"></div>
  {/if}

  <div class="pdf-body">
    <div class="pdf-pages" bind:this={container}></div>

    {#if loading}
      <div class="overlay">
        <div class="spinner"></div>
        <div class="hint">
          {$t('common.loading') || 'Loading...'}
          <span class="muted">({stage})</span>
        </div>
      </div>
    {:else if error}
      <div class="overlay error">
        <Icon name="alert-circle" size={20} />
        <div class="hint">
          {$t('components.lightbox.errors.load_failed') || 'Failed to load content.'}
          {#if error}
            <span class="muted">({error})</span>
          {/if}
        </div>
        <button
          class="btn primary"
          type="button"
          onclick={() => downloadFile(downloadUrl, filename)}
        >
          <Icon name="download" size={16} />
          {$t('components.lightbox.download_pdf') || 'Download PDF'}
        </button>
      </div>
    {/if}
  </div>

  {#if !loading && !error && blankWarning}
    <div class="blank-banner" role="status">
      <Icon name="alert-triangle" size={16} />
      <span class="text">{blankWarning}</span>
      <button class="btn tiny" type="button" onclick={() => downloadFile(downloadUrl, filename)}>
        <Icon name="download" size={14} />
        {$t('common.download') || 'Download'}
      </button>
    </div>
  {/if}
</div>

<style>
  .pdf-shell {
    width: 85vw;
    height: 80vh;
    border-radius: 12px;
    overflow: hidden;
    background: #0b1220;
    border: 1px solid rgba(255, 255, 255, 0.12);
    display: grid;
    grid-template-rows: auto 1fr;
  }

  .pdf-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.75rem 0.85rem;
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.55), rgba(0, 0, 0, 0.05));
    backdrop-filter: blur(6px);
  }

  .pdf-toolbar.hidden {
    padding: 0;
    height: 0;
    min-height: 0;
    overflow: hidden;
    border: 0;
    background: transparent;
    backdrop-filter: none;
  }

  .left {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: rgba(255, 255, 255, 0.85);
    font-weight: 800;
    letter-spacing: 0.02em;
  }

  .label {
    font-size: 0.9rem;
  }

  .sep {
    width: 1px;
    height: 16px;
    background: rgba(255, 255, 255, 0.18);
    margin: 0 0.25rem;
  }

  .zoom {
    font-size: 0.85rem;
    color: rgba(255, 255, 255, 0.7);
  }

  .right {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.9);
    padding: 0.4rem 0.6rem;
    cursor: pointer;
    font-weight: 800;
  }

  .btn:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .btn.primary {
    background: rgba(99, 102, 241, 0.25);
    border-color: rgba(99, 102, 241, 0.4);
  }

  .pdf-pages {
    overflow: auto;
    padding: 1rem;
    display: grid;
    gap: 0.9rem;
    justify-items: center;
    height: 100%;
    min-height: 0;
    overscroll-behavior: contain;
  }

  .pdf-body {
    position: relative;
    min-height: 0;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    gap: 0.65rem;
    padding: 2.5rem 1rem;
    color: rgba(255, 255, 255, 0.85);
    text-align: center;
    background: rgba(11, 18, 32, 0.55);
    backdrop-filter: blur(4px);
  }

  .overlay.error {
    background: rgba(11, 18, 32, 0.75);
  }

  .blank-banner {
    position: absolute;
    left: 0.85rem;
    bottom: 0.85rem;
    z-index: 5;
    display: inline-flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.55rem 0.65rem;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: rgba(11, 18, 32, 0.82);
    color: rgba(255, 255, 255, 0.9);
    backdrop-filter: blur(6px);
    max-width: min(720px, calc(100% - 1.7rem));
  }

  .blank-banner .text {
    font-weight: 800;
    color: rgba(255, 255, 255, 0.85);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn.tiny {
    padding: 0.3rem 0.45rem;
    border-radius: 10px;
    font-size: 0.85rem;
  }

  :global(.pdf-canvas) {
    max-width: 100%;
    height: auto;
    border-radius: 8px;
    background: white;
    box-shadow: 0 18px 55px rgba(0, 0, 0, 0.55);
  }

  .hint {
    font-weight: 800;
  }

  .muted {
    color: rgba(255, 255, 255, 0.6);
    font-weight: 700;
  }

  .spinner {
    width: 30px;
    height: 30px;
    border: 3px solid rgba(255, 255, 255, 0.16);
    border-top-color: rgba(99, 102, 241, 0.9);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 900px) {
    .pdf-shell {
      width: 92vw;
      height: 78vh;
    }
  }
</style>

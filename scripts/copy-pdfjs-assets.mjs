import fs from 'node:fs/promises';
import path from 'node:path';

// Copies PDF.js runtime assets (cmaps + standard fonts) into SvelteKit `static/`
// so previews work in WebView2/Tauri without relying on dynamic fetches from node_modules.

const ROOT = process.cwd();
const SRC_CMAPS = path.join(ROOT, 'node_modules', 'pdfjs-dist', 'cmaps');
const SRC_FONTS = path.join(ROOT, 'node_modules', 'pdfjs-dist', 'standard_fonts');
const SRC_WASM = path.join(ROOT, 'node_modules', 'pdfjs-dist', 'wasm');
const OUT_DIR = path.join(ROOT, 'static', 'pdfjs');
const OUT_CMAPS = path.join(OUT_DIR, 'cmaps');
const OUT_FONTS = path.join(OUT_DIR, 'standard_fonts');
const OUT_WASM = path.join(OUT_DIR, 'wasm');

async function exists(p) {
  try {
    await fs.access(p);
    return true;
  } catch {
    return false;
  }
}

async function copyDir(src, dst) {
  await fs.mkdir(dst, { recursive: true });
  // Node 18+: fs.cp is available. If not, this script should be updated.
  await fs.cp(src, dst, { recursive: true, force: true });
}

async function main() {
  if (!(await exists(SRC_CMAPS)) || !(await exists(SRC_FONTS)) || !(await exists(SRC_WASM))) {
    // pdfjs-dist might not be installed in some setups; don't fail installs.
    return;
  }

  await fs.mkdir(OUT_DIR, { recursive: true });
  await copyDir(SRC_CMAPS, OUT_CMAPS);
  await copyDir(SRC_FONTS, OUT_FONTS);
  await copyDir(SRC_WASM, OUT_WASM);
}

main().catch((e) => {
  console.error('[copy-pdfjs-assets] failed:', e);
  process.exitCode = 1;
});

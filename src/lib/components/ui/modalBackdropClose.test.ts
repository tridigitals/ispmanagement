import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('modal backdrop close behavior', () => {
  const cases: Array<{ path: string; desc: string }> = [
    {
      path: 'src/lib/components/ui/Modal.svelte',
      desc: 'base Modal default closeOnBackdropClick=false',
    },
    {
      path: 'src/lib/components/ui/ConfirmDialog.svelte',
      desc: 'ConfirmDialog',
    },
    {
      path: 'src/lib/components/ui/Lightbox.svelte',
      desc: 'Lightbox',
    },
    {
      path: 'src/lib/components/network/NetworkMapAssetCustomerDropModal.svelte',
      desc: 'NetworkMapAssetCustomerDropModal',
    },
  ];

  it.each(cases)('$desc — backdrop click must NOT close the modal', ({ path }) => {
    const src = readSource(path);
    // Ambil blok elemen backdrop/overlay
    const backdropTag = src.match(
      /<div[^>]*class\s*=\s*["\'][^"\']*\b(?:backdrop|overlay)\b[^"\']*["\'][^>]*>/,
    );
    expect(backdropTag).not.toBeNull();
    // Cek: jangan ada handler onclick pada backdrop yang memanggil close/hide/cancel
    expect(backdropTag![0]).not.toMatch(/onclick\s*=\s*\{?\s*(?:close|handleCancel|onClose|cancel|hide)\b/);
    expect(backdropTag![0]).not.toMatch(/e\s*\.\s*target\s*===/);
  });

  it('ConfirmDialog backdrop is a plain presentation div (no dismiss on click)', () => {
    const src = readSource('src/lib/components/ui/ConfirmDialog.svelte');
    expect(src).toContain('class="confirm-backdrop"');
    expect(src).toContain('onclick={(e) => e.stopPropagation()}');
    expect(src).not.toContain('e.target === e.currentTarget && handleCancel()');
  });
});

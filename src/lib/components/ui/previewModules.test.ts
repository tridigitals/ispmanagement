import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  PdfViewer: { name: 'pdf-viewer-component' },
  OfficeViewer: { name: 'office-viewer-component' },
}));

vi.mock('$lib/components/ui/PdfViewer.svelte', () => ({
  default: sentinels.PdfViewer,
}));

vi.mock('$lib/components/ui/OfficeViewer.svelte', () => ({
  default: sentinels.OfficeViewer,
}));

import { loadPreviewViewerModules } from './previewModules';

describe('preview viewer modules', () => {
  it('loads and caches the heavy preview viewers on demand', async () => {
    const first = await loadPreviewViewerModules();
    const second = await loadPreviewViewerModules();

    expect(first).toEqual({
      PdfViewerComponent: sentinels.PdfViewer,
      OfficeViewerComponent: sentinels.OfficeViewer,
    });
    expect(second).toBe(first);
  });
});

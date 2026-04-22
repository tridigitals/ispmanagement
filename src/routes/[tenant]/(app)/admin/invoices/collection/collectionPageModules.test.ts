import { describe, expect, it } from 'vitest';

import { loadCollectionExportModule } from './collectionPageModules';

describe('collection page modules', () => {
  it('loads and caches the export helpers lazily', async () => {
    const first = await loadCollectionExportModule();
    const second = await loadCollectionExportModule();

    expect(typeof first.exportCsvRows).toBe('function');
    expect(typeof first.exportExcelRows).toBe('function');
    expect(second).toBe(first);
  });
});

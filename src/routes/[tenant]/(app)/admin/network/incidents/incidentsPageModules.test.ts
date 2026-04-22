import { describe, expect, it } from 'vitest';

import { loadIncidentsExportModule } from './incidentsPageModules';

describe('incidents page modules', () => {
  it('loads and caches the export helpers lazily', async () => {
    const first = await loadIncidentsExportModule();
    const second = await loadIncidentsExportModule();

    expect(typeof first.exportCsvRows).toBe('function');
    expect(typeof first.exportExcelRows).toBe('function');
    expect(second).toBe(first);
  });
});

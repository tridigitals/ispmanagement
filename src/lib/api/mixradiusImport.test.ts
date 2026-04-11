import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('mixradius import api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('uploads a MixRadius backup with file metadata and optional local path', async () => {
    safeInvoke.mockResolvedValue({ id: 'batch-1' });

    const { mixradiusImport } = await import('./mixradiusImport');
    await mixradiusImport.upload({
      file_name: 'MixRadius.sql.gz',
      file_size_bytes: 1024,
      content_type: 'application/gzip',
      source_checksum: 'sha256',
      local_path: '/tmp/MixRadius.sql.gz',
    });

    expect(safeInvoke).toHaveBeenCalledWith('upload_mixradius_import', {
      token: 'token-123',
      fileName: 'MixRadius.sql.gz',
      file_name: 'MixRadius.sql.gz',
      fileSizeBytes: 1024,
      file_size_bytes: 1024,
      contentType: 'application/gzip',
      content_type: 'application/gzip',
      sourceChecksum: 'sha256',
      source_checksum: 'sha256',
      localPath: '/tmp/MixRadius.sql.gz',
      local_path: '/tmp/MixRadius.sql.gz',
    });
  });

  it('lists and gets import batches through stable route keys', async () => {
    safeInvoke.mockResolvedValueOnce({ data: [], total: 0, page: 1, per_page: 25 });
    safeInvoke.mockResolvedValueOnce({ id: 'batch-1' });

    const { mixradiusImport } = await import('./mixradiusImport');
    await mixradiusImport.list({ page: 2, per_page: 50, status: 'pending' });
    await mixradiusImport.get('batch-1');

    expect(safeInvoke).toHaveBeenNthCalledWith(1, 'list_mixradius_import_batches', {
      token: 'token-123',
      page: 2,
      per_page: 50,
      status: 'pending',
    });
    expect(safeInvoke).toHaveBeenNthCalledWith(2, 'get_mixradius_import_batch', {
      token: 'token-123',
      batchId: 'batch-1',
      batch_id: 'batch-1',
    });
  });

  it('passes mapping overrides and conflict decisions through preview and execute calls', async () => {
    safeInvoke.mockResolvedValueOnce({ batch_id: 'batch-1', rows: [] });
    safeInvoke.mockResolvedValueOnce({ summary: { imported_rows: 0 } });

    const overrides = [
      {
        source_kind: 'nas',
        source_value: '5',
        target_kind: 'router',
        target_value: 'router-1',
      },
    ];
    const { mixradiusImport } = await import('./mixradiusImport');
    await mixradiusImport.preview({
      batch_id: 'batch-1',
      mapping_overrides: overrides,
      customer_conflict_resolution: 'merge',
      location_strategy: 'preserve',
    });
    await mixradiusImport.execute({
      batch_id: 'batch-1',
      execution_mode: 'safe_import',
      mapping_overrides: overrides,
      customer_conflict_resolution: 'skip',
      location_strategy: 'merge',
    });

    expect(safeInvoke).toHaveBeenNthCalledWith(1, 'preview_mixradius_import', {
      token: 'token-123',
      batchId: 'batch-1',
      batch_id: 'batch-1',
      mappingOverrides: overrides,
      mapping_overrides: overrides,
      customerConflictResolution: 'merge',
      customer_conflict_resolution: 'merge',
      locationStrategy: 'preserve',
      location_strategy: 'preserve',
    });
    expect(safeInvoke).toHaveBeenNthCalledWith(2, 'execute_mixradius_import', {
      token: 'token-123',
      batchId: 'batch-1',
      batch_id: 'batch-1',
      executionMode: 'safe_import',
      execution_mode: 'safe_import',
      mappingOverrides: overrides,
      mapping_overrides: overrides,
      customerConflictResolution: 'skip',
      customer_conflict_resolution: 'skip',
      locationStrategy: 'merge',
      location_strategy: 'merge',
    });
  });

  it('cancels an import batch through the cancel route key', async () => {
    safeInvoke.mockResolvedValue({ ok: true });

    const { mixradiusImport } = await import('./mixradiusImport');
    await mixradiusImport.cancel('batch-1');

    expect(safeInvoke).toHaveBeenCalledWith('cancel_mixradius_import', {
      token: 'token-123',
      batchId: 'batch-1',
      batch_id: 'batch-1',
    });
  });
});

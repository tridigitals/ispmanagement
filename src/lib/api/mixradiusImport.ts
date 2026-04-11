import { getTokenOrThrow, safeInvoke } from './core';

export type MixradiusImportConflictResolution = 'merge' | 'create_new' | 'skip';
export type MixradiusImportLocationStrategy = 'preserve' | 'merge' | 'replace';
export type MixradiusImportExecutionMode = 'preview_only' | 'safe_import' | 'force_sync';

export interface MixradiusImportMappingOverrideInput {
  source_kind: string;
  source_value: string;
  target_kind: string;
  target_value: string;
}

export interface MixradiusImportUploadInput {
  file_name: string;
  file_size_bytes: number;
  content_type?: string | null;
  source_checksum?: string | null;
  local_path?: string | null;
}

export interface MixradiusImportPreviewInput {
  batch_id: string;
  mapping_overrides?: MixradiusImportMappingOverrideInput[];
  customer_conflict_resolution?: MixradiusImportConflictResolution | null;
  location_strategy?: MixradiusImportLocationStrategy | null;
}

export interface MixradiusImportExecuteInput extends MixradiusImportPreviewInput {
  execution_mode: MixradiusImportExecutionMode;
}

export const mixradiusImport = {
  upload: (dto: MixradiusImportUploadInput): Promise<any> =>
    safeInvoke('upload_mixradius_import', {
      token: getTokenOrThrow(),
      fileName: dto.file_name,
      file_name: dto.file_name,
      fileSizeBytes: dto.file_size_bytes,
      file_size_bytes: dto.file_size_bytes,
      contentType: dto.content_type ?? undefined,
      content_type: dto.content_type ?? undefined,
      sourceChecksum: dto.source_checksum ?? undefined,
      source_checksum: dto.source_checksum ?? undefined,
      localPath: dto.local_path ?? undefined,
      local_path: dto.local_path ?? undefined,
    }),

  list: (params?: {
    page?: number;
    per_page?: number;
    status?: string;
  }): Promise<any> =>
    safeInvoke('list_mixradius_import_batches', {
      token: getTokenOrThrow(),
      page: params?.page,
      per_page: params?.per_page,
      status: params?.status,
    }),

  get: (batchId: string): Promise<any> =>
    safeInvoke('get_mixradius_import_batch', {
      token: getTokenOrThrow(),
      batchId,
      batch_id: batchId,
    }),

  preview: (dto: MixradiusImportPreviewInput): Promise<any> =>
    safeInvoke('preview_mixradius_import', {
      token: getTokenOrThrow(),
      batchId: dto.batch_id,
      batch_id: dto.batch_id,
      mappingOverrides: dto.mapping_overrides ?? [],
      mapping_overrides: dto.mapping_overrides ?? [],
      customerConflictResolution: dto.customer_conflict_resolution ?? undefined,
      customer_conflict_resolution: dto.customer_conflict_resolution ?? undefined,
      locationStrategy: dto.location_strategy ?? undefined,
      location_strategy: dto.location_strategy ?? undefined,
    }),

  execute: (dto: MixradiusImportExecuteInput): Promise<any> =>
    safeInvoke('execute_mixradius_import', {
      token: getTokenOrThrow(),
      batchId: dto.batch_id,
      batch_id: dto.batch_id,
      executionMode: dto.execution_mode,
      execution_mode: dto.execution_mode,
      mappingOverrides: dto.mapping_overrides ?? [],
      mapping_overrides: dto.mapping_overrides ?? [],
      customerConflictResolution: dto.customer_conflict_resolution ?? undefined,
      customer_conflict_resolution: dto.customer_conflict_resolution ?? undefined,
      locationStrategy: dto.location_strategy ?? undefined,
      location_strategy: dto.location_strategy ?? undefined,
    }),

  cancel: (batchId: string): Promise<any> =>
    safeInvoke('cancel_mixradius_import', {
      token: getTokenOrThrow(),
      batchId,
      batch_id: batchId,
    }),
};

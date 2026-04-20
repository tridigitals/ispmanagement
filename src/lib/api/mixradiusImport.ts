import { getTokenOrThrow, safeInvoke } from './core';
import { getApiBaseUrl } from '$lib/utils/apiUrl';
import type {
  MixradiusImportBatch,
  MixradiusImportConflictResolution,
  MixradiusImportExecutionMode,
  MixradiusImportExecutionResult,
  MixradiusImportLocationStrategy,
  MixradiusImportMappingOverrideInput,
  MixradiusImportPppoeProvisioningTarget,
  MixradiusImportPreview,
} from '$lib/components/network/mixradius/mixradiusImportTypes';

export interface MixradiusImportUploadInput {
  file_name: string;
  file_size_bytes: number;
  content_type?: string | null;
  source_checksum?: string | null;
  local_path?: string | null;
  file?: File | null;
}

export interface MixradiusImportPreviewInput {
  batch_id: string;
  mapping_overrides?: MixradiusImportMappingOverrideInput[];
  customer_conflict_resolution?: MixradiusImportConflictResolution | null;
  location_strategy?: MixradiusImportLocationStrategy | null;
  pppoe_provisioning_target?: MixradiusImportPppoeProvisioningTarget | null;
}

export interface MixradiusImportExecuteInput extends MixradiusImportPreviewInput {
  execution_mode: MixradiusImportExecutionMode;
}

export const mixradiusImport = {
  upload: async (dto: MixradiusImportUploadInput): Promise<MixradiusImportBatch> => {
    const token = getTokenOrThrow();

    if (dto.file) {
      const formData = new FormData();
      formData.append('file', dto.file);
      formData.append('file_name', dto.file_name);
      formData.append('file_size_bytes', String(dto.file_size_bytes));
      if (dto.content_type) formData.append('content_type', dto.content_type);
      if (dto.source_checksum) formData.append('source_checksum', dto.source_checksum);

      const response = await fetch(`${getApiBaseUrl()}/admin/pppoe/mixradius/imports/upload`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` },
        body: formData,
      });

      if (!response.ok) {
        const raw = await response.text().catch(() => '');
        let message = 'Gagal upload backup MixRadius';
        if (raw) {
          try {
            const errorBody = JSON.parse(raw);
            message =
              errorBody?.error || errorBody?.message || errorBody?.detail || errorBody?.details || raw;
          } catch {
            message = raw;
          }
        }
        throw new Error(message);
      }

      return (await response.json()) as MixradiusImportBatch;
    }

    return await safeInvoke('upload_mixradius_import', {
      token,
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
    });
  },

  list: (params?: {
    page?: number;
    per_page?: number;
    status?: string;
  }): Promise<{ data: MixradiusImportBatch[]; total: number; page: number; per_page: number }> =>
    safeInvoke('list_mixradius_import_batches', {
      token: getTokenOrThrow(),
      page: params?.page,
      per_page: params?.per_page,
      status: params?.status,
      __suppress_error_log: true,
    }),

  get: (batchId: string): Promise<MixradiusImportBatch> =>
    safeInvoke('get_mixradius_import_batch', {
      token: getTokenOrThrow(),
      batchId,
      batch_id: batchId,
    }),

  preview: (dto: MixradiusImportPreviewInput): Promise<MixradiusImportPreview> =>
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
      pppoeProvisioningTarget: dto.pppoe_provisioning_target ?? undefined,
      pppoe_provisioning_target: dto.pppoe_provisioning_target ?? undefined,
    }),

  execute: (dto: MixradiusImportExecuteInput): Promise<MixradiusImportExecutionResult> =>
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
      pppoeProvisioningTarget: dto.pppoe_provisioning_target ?? undefined,
      pppoe_provisioning_target: dto.pppoe_provisioning_target ?? undefined,
      __timeout_ms: 900000,
    }),

  cancel: (batchId: string): Promise<MixradiusImportBatch> =>
    safeInvoke('cancel_mixradius_import', {
      token: getTokenOrThrow(),
      batchId,
      batch_id: batchId,
    }),
};

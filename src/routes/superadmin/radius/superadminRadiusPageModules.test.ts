import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  assignmentFormModal: { name: 'radius-assignment-form-modal' },
  mappingFormModal: { name: 'radius-mapping-form-modal' },
  mappingSecretDialog: { name: 'radius-mapping-secret-dialog' },
  serverFormModal: { name: 'radius-server-form-modal' },
}));

vi.mock('$lib/components/superadmin/radius/AssignmentFormModal.svelte', () => ({
  default: sentinels.assignmentFormModal,
}));

vi.mock('$lib/components/superadmin/radius/MappingFormModal.svelte', () => ({
  default: sentinels.mappingFormModal,
}));

vi.mock('$lib/components/superadmin/radius/MappingSecretDialog.svelte', () => ({
  default: sentinels.mappingSecretDialog,
}));

vi.mock('$lib/components/superadmin/radius/ServerFormModal.svelte', () => ({
  default: sentinels.serverFormModal,
}));

import { loadSuperadminRadiusDialogs } from './superadminRadiusPageModules';

describe('superadmin radius page modules', () => {
  it('loads and caches the managed radius dialogs lazily', async () => {
    const first = await loadSuperadminRadiusDialogs();
    const second = await loadSuperadminRadiusDialogs();

    expect(first.AssignmentFormModalComponent).toBe(sentinels.assignmentFormModal);
    expect(first.MappingFormModalComponent).toBe(sentinels.mappingFormModal);
    expect(first.MappingSecretDialogComponent).toBe(sentinels.mappingSecretDialog);
    expect(first.ServerFormModalComponent).toBe(sentinels.serverFormModal);
    expect(second).toBe(first);
  });
});

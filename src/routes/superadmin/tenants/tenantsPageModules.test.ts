import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  tenantTable: { name: 'tenant-table-component' },
  tenantFormModal: { name: 'tenant-form-modal-component' },
  confirmDialog: { name: 'confirm-dialog-component' },
}));

vi.mock('$lib/components/superadmin/tenants/TenantTable.svelte', () => ({
  default: sentinels.tenantTable,
}));

vi.mock('$lib/components/superadmin/tenants/TenantFormModal.svelte', () => ({
  default: sentinels.tenantFormModal,
}));

vi.mock('$lib/components/ui/ConfirmDialog.svelte', () => ({
  default: sentinels.confirmDialog,
}));

import { loadSuperadminTenantsModules } from './tenantsPageModules';

describe('superadmin tenants page modules', () => {
  it('loads and caches tenants modules lazily', async () => {
    const first = await loadSuperadminTenantsModules();
    const second = await loadSuperadminTenantsModules();

    expect(first.TenantTableComponent).toBe(sentinels.tenantTable);
    expect(first.TenantFormModalComponent).toBe(sentinels.tenantFormModal);
    expect(first.ConfirmDialogComponent).toBe(sentinels.confirmDialog);
    expect(second).toBe(first);
  });
});

import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  Modal: { name: 'modal-component' },
  Select2: { name: 'select2-component' },
  Toggle: { name: 'toggle-component' },
  ConfirmDialog: { name: 'confirm-dialog-component' },
}));

vi.mock('$lib/components/ui/Modal.svelte', () => ({
  default: sentinels.Modal,
}));

vi.mock('$lib/components/ui/Select2.svelte', () => ({
  default: sentinels.Select2,
}));

vi.mock('$lib/components/ui/Toggle.svelte', () => ({
  default: sentinels.Toggle,
}));

vi.mock('$lib/components/ui/ConfirmDialog.svelte', () => ({
  default: sentinels.ConfirmDialog,
}));

import { loadCustomerDetailDialogModules } from './customerDetailModules';

describe('customer detail modules', () => {
  it('loads and caches the dialog UI modules on demand', async () => {
    const first = await loadCustomerDetailDialogModules();
    const second = await loadCustomerDetailDialogModules();

    expect(first).toEqual({
      ModalComponent: sentinels.Modal,
      Select2Component: sentinels.Select2,
      ToggleComponent: sentinels.Toggle,
      ConfirmDialogComponent: sentinels.ConfirmDialog,
    });
    expect(second).toBe(first);
  });
});

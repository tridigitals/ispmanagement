import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  modal: { name: 'team-modal-component' },
  confirmDialog: { name: 'team-confirm-dialog-component' },
}));

vi.mock('$lib/components/ui/Modal.svelte', () => ({
  default: sentinels.modal,
}));

vi.mock('$lib/components/ui/ConfirmDialog.svelte', () => ({
  default: sentinels.confirmDialog,
}));

import { loadTeamDialogModules } from './teamPageModules';

describe('team page modules', () => {
  it('loads and caches dialog modules lazily', async () => {
    const first = await loadTeamDialogModules();
    const second = await loadTeamDialogModules();

    expect(first.ModalComponent).toBe(sentinels.modal);
    expect(first.ConfirmDialogComponent).toBe(sentinels.confirmDialog);
    expect(second).toBe(first);
  });
});

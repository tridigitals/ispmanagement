import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  Modal: { name: 'modal-component' },
  Select2: { name: 'select2-component' },
  Toggle: { name: 'toggle-component' },
  ServicesDialogs: { name: 'services-dialogs-component' },
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

vi.mock('./ServicesDialogs.svelte', () => ({
  default: sentinels.ServicesDialogs,
}));

import { loadServicesDialogs, loadServicesModalModules } from './servicesPageModules';

describe('services page modules', () => {
  it('loads and caches the modal-related UI components on demand', async () => {
    const first = await loadServicesModalModules();
    const second = await loadServicesModalModules();

    expect(first).toEqual({
      ModalComponent: sentinels.Modal,
      Select2Component: sentinels.Select2,
      ToggleComponent: sentinels.Toggle,
    });
    expect(second).toBe(first);
  });

  it('loads and caches the services dialogs lazily', async () => {
    const first = await loadServicesDialogs();
    const second = await loadServicesDialogs();

    expect(first.ServicesDialogsComponent).toBe(sentinels.ServicesDialogs);
    expect(second).toBe(first);
  });
});

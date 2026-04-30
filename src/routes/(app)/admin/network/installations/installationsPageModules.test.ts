import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  select2: { name: 'installations-select2' },
  cableMap: { name: 'installation-cable-map' },
  detailDialogs: { name: 'installation-detail-dialogs' },
}));

vi.mock('$lib/components/ui/Select2.svelte', () => ({
  default: sentinels.select2,
}));

vi.mock('$lib/components/network/InstallationCableMap.svelte', () => ({
  default: sentinels.cableMap,
}));

vi.mock('./InstallationDetailDialogs.svelte', () => ({
  default: sentinels.detailDialogs,
}));

import {
  loadInstallationDetailDialogs,
  loadInstallationDetailModules,
} from './installationsPageModules';

describe('installations page modules', () => {
  it('loads and caches detail-only modules lazily', async () => {
    const first = await loadInstallationDetailModules();
    const second = await loadInstallationDetailModules();

    expect(first.Select2Component).toBe(sentinels.select2);
    expect(first.InstallationCableMapComponent).toBe(sentinels.cableMap);
    expect(second).toBe(first);
  });

  it('loads and caches the installation detail dialogs lazily', async () => {
    const first = await loadInstallationDetailDialogs();
    const second = await loadInstallationDetailDialogs();

    expect(first.InstallationDetailDialogsComponent).toBe(sentinels.detailDialogs);
    expect(second).toBe(first);
  });
});

import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  general: { name: 'settings-general-tab' },
  auth: { name: 'settings-auth-tab' },
  password: { name: 'settings-password-tab' },
  security: { name: 'settings-security-tab' },
  storage: { name: 'settings-storage-tab' },
  payment: { name: 'settings-payment-tab' },
  alerting: { name: 'settings-alerting-tab' },
  backup: { name: 'settings-backup-tab' },
}));

vi.mock('$lib/components/superadmin/settings/SettingsGeneralTab.svelte', () => ({
  default: sentinels.general,
}));

vi.mock('$lib/components/superadmin/settings/SettingsAuthTab.svelte', () => ({
  default: sentinels.auth,
}));

vi.mock('$lib/components/superadmin/settings/SettingsPasswordTab.svelte', () => ({
  default: sentinels.password,
}));

vi.mock('$lib/components/superadmin/settings/SettingsSecurityTab.svelte', () => ({
  default: sentinels.security,
}));

vi.mock('$lib/components/superadmin/settings/SettingsStorageTab.svelte', () => ({
  default: sentinels.storage,
}));

vi.mock('$lib/components/superadmin/settings/SettingsPaymentTab.svelte', () => ({
  default: sentinels.payment,
}));

vi.mock('$lib/components/superadmin/settings/SettingsAlertingTab.svelte', () => ({
  default: sentinels.alerting,
}));

vi.mock('$lib/components/superadmin/settings/SettingsBackupTab.svelte', () => ({
  default: sentinels.backup,
}));

import {
  loadSettingsAlertingTab,
  loadSettingsAuthTab,
  loadSettingsBackupTab,
  loadSettingsGeneralTab,
  loadSettingsPasswordTab,
  loadSettingsPaymentTab,
  loadSettingsSecurityTab,
  loadSettingsStorageTab,
} from './settingsPageModules';

describe('superadmin settings page modules', () => {
  it('loads and caches each settings tab on demand', async () => {
    const general = await loadSettingsGeneralTab();
    const generalCached = await loadSettingsGeneralTab();
    const auth = await loadSettingsAuthTab();
    const password = await loadSettingsPasswordTab();
    const security = await loadSettingsSecurityTab();
    const storage = await loadSettingsStorageTab();
    const payment = await loadSettingsPaymentTab();
    const alerting = await loadSettingsAlertingTab();
    const backup = await loadSettingsBackupTab();

    expect(general).toEqual({ default: sentinels.general });
    expect(generalCached).toBe(general);
    expect(auth).toEqual({ default: sentinels.auth });
    expect(password).toEqual({ default: sentinels.password });
    expect(security).toEqual({ default: sentinels.security });
    expect(storage).toEqual({ default: sentinels.storage });
    expect(payment).toEqual({ default: sentinels.payment });
    expect(alerting).toEqual({ default: sentinels.alerting });
    expect(backup).toEqual({ default: sentinels.backup });
  });
});

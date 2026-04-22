type DeferredComponent = any;
type AsyncModuleLoader<T> = () => Promise<T>;

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export const loadSettingsGeneralTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsGeneralTab.svelte'),
);

export const loadSettingsAuthTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsAuthTab.svelte'),
);

export const loadSettingsPasswordTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsPasswordTab.svelte'),
);

export const loadSettingsSecurityTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsSecurityTab.svelte'),
);

export const loadSettingsStorageTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsStorageTab.svelte'),
);

export const loadSettingsPaymentTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsPaymentTab.svelte'),
);

export const loadSettingsAlertingTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsAlertingTab.svelte'),
);

export const loadSettingsBackupTab = createCachedLoader(() =>
  import('$lib/components/superadmin/settings/SettingsBackupTab.svelte'),
);

export type SettingsTabId =
  | 'general'
  | 'auth'
  | 'password'
  | 'security'
  | 'storage'
  | 'payment'
  | 'alerting'
  | 'backup';

type SettingsTabLoader = () => Promise<{ default: DeferredComponent }>;

const settingsTabLoaders: Record<SettingsTabId, SettingsTabLoader> = {
  general: loadSettingsGeneralTab,
  auth: loadSettingsAuthTab,
  password: loadSettingsPasswordTab,
  security: loadSettingsSecurityTab,
  storage: loadSettingsStorageTab,
  payment: loadSettingsPaymentTab,
  alerting: loadSettingsAlertingTab,
  backup: loadSettingsBackupTab,
};

export async function loadSettingsTabComponent(tab: SettingsTabId): Promise<DeferredComponent> {
  const module = await settingsTabLoaders[tab]();
  return module.default;
}

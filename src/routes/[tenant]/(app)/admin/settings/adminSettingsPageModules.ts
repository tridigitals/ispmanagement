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

export const loadTenantBillingPlanPanel = createCachedLoader(async () => {
  const { default: BillingPlanPanelComponent } = await import(
    '$lib/components/billing/TenantBillingPlanPanel.svelte'
  );

  return {
    BillingPlanPanelComponent,
  };
});

export const loadAdminSettingsEmailTab = createCachedLoader(async () => {
  const { default: SettingsEmailTabComponent } = await import('./SettingsEmailTab.svelte');

  return {
    SettingsEmailTabComponent,
  };
});

export const loadAdminSettingsPaymentTab = createCachedLoader(async () => {
  const { default: SettingsPaymentTabComponent } = await import('./SettingsPaymentTab.svelte');

  return {
    SettingsPaymentTabComponent,
  };
});

export const loadAdminSettingsWhatsAppTab = createCachedLoader(async () => {
  const { default: SettingsWhatsAppTabComponent } = await import(
    '$lib/components/settings/WhatsAppGatewayTab.svelte'
  );

  return {
    SettingsWhatsAppTabComponent,
  };
});

export const loadAdminSettingsNotificationEventsTab = createCachedLoader(async () => {
  const { default: SettingsNotificationEventsTabComponent } = await import(
    '$lib/components/settings/NotificationEventsTab.svelte'
  );

  return {
    SettingsNotificationEventsTabComponent,
  };
});

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

export const loadCustomerSubscriptionsTab = createCachedLoader(() =>
  import('./CustomerSubscriptionsTab.svelte'),
);

export const loadCustomerBillingTab = createCachedLoader(() =>
  import('./CustomerBillingTab.svelte'),
);

export const loadCustomerPppoeTab = createCachedLoader(() =>
  import('./CustomerPppoeTab.svelte'),
);

export const loadCustomerTimelineTab = createCachedLoader(() =>
  import('./CustomerTimelineTab.svelte'),
);

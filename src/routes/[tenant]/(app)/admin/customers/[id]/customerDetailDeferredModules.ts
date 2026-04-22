import type { buildCustomerTimelineRows as buildCustomerTimelineRowsValue } from '$lib/utils/customerTimelineTable';
import type {
  getPppoeApplyActionFallback as getPppoeApplyActionFallbackValue,
  getPppoeProvisioningTargetFallback as getPppoeProvisioningTargetFallbackValue,
  getPppoeSyncDisplay as getPppoeSyncDisplayValue,
} from '$lib/utils/pppoeSource';
import type { getCustomerPppoeToolbarConfig as getCustomerPppoeToolbarConfigValue } from '$lib/utils/customerPppoeToolbar';

type AsyncModuleLoader<T> = () => Promise<T>;

type CustomerPppoeToolbar = ReturnType<typeof getCustomerPppoeToolbarConfigValue>;

export type CustomerPppoeHelperModule = {
  pppoeToolbar: CustomerPppoeToolbar;
  getPppoeSyncDisplay: typeof getPppoeSyncDisplayValue;
  getPppoeProvisioningTargetFallback: typeof getPppoeProvisioningTargetFallbackValue;
  getPppoeApplyActionFallback: typeof getPppoeApplyActionFallbackValue;
};

export type CustomerTimelineHelperModule = {
  buildCustomerTimelineRows: typeof buildCustomerTimelineRowsValue;
};

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export const loadCustomerPppoeHelperModule = createCachedLoader<CustomerPppoeHelperModule>(
  async () => {
    const [pppoeSource, toolbar] = await Promise.all([
      import('$lib/utils/pppoeSource'),
      import('$lib/utils/customerPppoeToolbar'),
    ]);

    return {
      pppoeToolbar: toolbar.getCustomerPppoeToolbarConfig(),
      getPppoeSyncDisplay: pppoeSource.getPppoeSyncDisplay,
      getPppoeProvisioningTargetFallback: pppoeSource.getPppoeProvisioningTargetFallback,
      getPppoeApplyActionFallback: pppoeSource.getPppoeApplyActionFallback,
    };
  },
);

export const loadCustomerTimelineHelperModule = createCachedLoader<CustomerTimelineHelperModule>(
  async () => {
    const module = await import('$lib/utils/customerTimelineTable');

    return {
      buildCustomerTimelineRows: module.buildCustomerTimelineRows,
    };
  },
);

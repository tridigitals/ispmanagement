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

export type SuperadminSystemHealthModules = {
  SystemStatusBannerComponent: DeferredComponent;
  SystemResourcesComponent: DeferredComponent;
  RequestMetricsComponent: DeferredComponent;
  SystemStatsGridComponent: DeferredComponent;
  DatabaseTablesComponent: DeferredComponent;
  RecentActivityComponent: DeferredComponent;
};

export type SuperadminSystemDiagnosticsModules = {
  SystemDiagnosticsPanelComponent: DeferredComponent;
};

export const loadSuperadminSystemHealthModules =
  createCachedLoader<SuperadminSystemHealthModules>(async () => {
    const [
      { default: SystemStatusBannerComponent },
      { default: SystemResourcesComponent },
      { default: RequestMetricsComponent },
      { default: SystemStatsGridComponent },
      { default: DatabaseTablesComponent },
      { default: RecentActivityComponent },
    ] = await Promise.all([
      import('$lib/components/superadmin/system/SystemStatusBanner.svelte'),
      import('$lib/components/superadmin/system/SystemResources.svelte'),
      import('$lib/components/superadmin/system/RequestMetrics.svelte'),
      import('$lib/components/superadmin/system/SystemStatsGrid.svelte'),
      import('$lib/components/superadmin/system/DatabaseTables.svelte'),
      import('$lib/components/superadmin/system/RecentActivity.svelte'),
    ]);

    return {
      SystemStatusBannerComponent,
      SystemResourcesComponent,
      RequestMetricsComponent,
      SystemStatsGridComponent,
      DatabaseTablesComponent,
      RecentActivityComponent,
    };
  });

export const loadSuperadminSystemDiagnosticsModules =
  createCachedLoader<SuperadminSystemDiagnosticsModules>(async () => {
    const { default: SystemDiagnosticsPanelComponent } = await import(
      '$lib/components/superadmin/system/SystemDiagnosticsPanel.svelte'
    );

    return {
      SystemDiagnosticsPanelComponent,
    };
  });

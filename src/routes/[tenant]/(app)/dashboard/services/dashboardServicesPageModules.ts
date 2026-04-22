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

export const loadDashboardServicesTrackerModal = createCachedLoader(async () => {
  const { default: TrackerModalComponent } = await import('./DashboardServicesTrackerModal.svelte');

  return {
    TrackerModalComponent,
  };
});

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

export const loadLocationFormModal = createCachedLoader(async () => {
  const { default: LocationFormModalComponent } = await import('./LocationFormModal.svelte');

  return {
    LocationFormModalComponent: LocationFormModalComponent as DeferredComponent,
  };
});

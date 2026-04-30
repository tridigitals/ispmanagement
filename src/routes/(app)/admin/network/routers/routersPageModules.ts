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

export const loadRouterFormModal = createCachedLoader(async () => {
  const { default: RouterFormModalComponent } = await import('./RouterFormModal.svelte');

  return {
    RouterFormModalComponent: RouterFormModalComponent as DeferredComponent,
  };
});

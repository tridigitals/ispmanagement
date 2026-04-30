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

export const loadRouterDetailDialogs = createCachedLoader(async () => {
  const { default: RouterDetailDialogsComponent } = await import('./RouterDetailDialogs.svelte');

  return {
    RouterDetailDialogsComponent: RouterDetailDialogsComponent as DeferredComponent,
  };
});

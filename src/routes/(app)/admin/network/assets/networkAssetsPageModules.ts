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

export const loadNetworkAssetFormModal = createCachedLoader(async () => {
  const { default: NetworkAssetFormModalComponent } = await import('./NetworkAssetFormModal.svelte');

  return {
    NetworkAssetFormModalComponent: NetworkAssetFormModalComponent as DeferredComponent,
  };
});

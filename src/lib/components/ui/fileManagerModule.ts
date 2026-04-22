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

export const loadFileManagerModule = createCachedLoader(async () => {
  const { default: FileManagerComponent } = await import('$lib/components/ui/FileManager.svelte');

  return {
    FileManagerComponent,
  };
});

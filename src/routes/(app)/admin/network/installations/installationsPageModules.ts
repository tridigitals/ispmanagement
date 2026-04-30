type DeferredComponent = any;
type AsyncModuleLoader<T> = () => Promise<T>;

export type InstallationDetailModules = {
  Select2Component: DeferredComponent;
  InstallationCableMapComponent: DeferredComponent;
};

export type InstallationDetailDialogsModule = {
  InstallationDetailDialogsComponent: DeferredComponent;
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

export const loadInstallationDetailModules = createCachedLoader(async () => {
  const [{ default: Select2Component }, { default: InstallationCableMapComponent }] =
    await Promise.all([
      import('$lib/components/ui/Select2.svelte'),
      import('$lib/components/network/InstallationCableMap.svelte'),
    ]);

  return {
    Select2Component,
    InstallationCableMapComponent,
  };
});

export const loadInstallationDetailDialogs =
  createCachedLoader<InstallationDetailDialogsModule>(async () => {
    const { default: InstallationDetailDialogsComponent } = await import(
      './InstallationDetailDialogs.svelte'
    );

    return {
      InstallationDetailDialogsComponent,
    };
  });

import type { Component } from 'svelte';

type DeferredComponent = Component<any>;
type AsyncModuleLoader<T> = () => Promise<T>;

export type ServicesModalModules = {
  ModalComponent: DeferredComponent;
  Select2Component: DeferredComponent;
  ToggleComponent: DeferredComponent;
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

export const loadServicesModalModules = createCachedLoader(async () => {
  const [{ default: ModalComponent }, { default: Select2Component }, { default: ToggleComponent }] =
    await Promise.all([
      import('$lib/components/ui/Modal.svelte'),
      import('$lib/components/ui/Select2.svelte'),
      import('$lib/components/ui/Toggle.svelte'),
    ]);

  return {
    ModalComponent,
    Select2Component,
    ToggleComponent,
  };
});

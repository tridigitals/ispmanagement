import type { Component } from 'svelte';

type DeferredComponent = Component<any>;
type AsyncModuleLoader<T> = () => Promise<T>;

export type CustomerDetailDialogModules = {
  ModalComponent: DeferredComponent;
  Select2Component: DeferredComponent;
  ToggleComponent: DeferredComponent;
  ConfirmDialogComponent: DeferredComponent;
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

export const loadCustomerDetailDialogModules = createCachedLoader(async () => {
  const [
    { default: ModalComponent },
    { default: Select2Component },
    { default: ToggleComponent },
    { default: ConfirmDialogComponent },
  ] = await Promise.all([
    import('$lib/components/ui/Modal.svelte'),
    import('$lib/components/ui/Select2.svelte'),
    import('$lib/components/ui/Toggle.svelte'),
    import('$lib/components/ui/ConfirmDialog.svelte'),
  ]);

  return {
    ModalComponent,
    Select2Component,
    ToggleComponent,
    ConfirmDialogComponent,
  };
});

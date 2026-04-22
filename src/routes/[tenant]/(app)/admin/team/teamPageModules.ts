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

export type TeamDialogModules = {
  ModalComponent: DeferredComponent;
  ConfirmDialogComponent: DeferredComponent;
};

export const loadTeamDialogModules = createCachedLoader<TeamDialogModules>(async () => {
  const [{ default: ModalComponent }, { default: ConfirmDialogComponent }] = await Promise.all([
    import('$lib/components/ui/Modal.svelte'),
    import('$lib/components/ui/ConfirmDialog.svelte'),
  ]);

  return {
    ModalComponent,
    ConfirmDialogComponent,
  };
});

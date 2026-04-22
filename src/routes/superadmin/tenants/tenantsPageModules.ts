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

export type SuperadminTenantsModules = {
  TenantTableComponent: DeferredComponent;
  TenantFormModalComponent: DeferredComponent;
  ConfirmDialogComponent: DeferredComponent;
};

export const loadSuperadminTenantsModules =
  createCachedLoader<SuperadminTenantsModules>(async () => {
    const [
      { default: TenantTableComponent },
      { default: TenantFormModalComponent },
      { default: ConfirmDialogComponent },
    ] = await Promise.all([
      import('$lib/components/superadmin/tenants/TenantTable.svelte'),
      import('$lib/components/superadmin/tenants/TenantFormModal.svelte'),
      import('$lib/components/ui/ConfirmDialog.svelte'),
    ]);

    return {
      TenantTableComponent,
      TenantFormModalComponent,
      ConfirmDialogComponent,
    };
  });

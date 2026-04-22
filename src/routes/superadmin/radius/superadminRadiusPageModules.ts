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

export const loadSuperadminRadiusDialogs = createCachedLoader(async () => {
  const [
    { default: AssignmentFormModalComponent },
    { default: MappingFormModalComponent },
    { default: MappingSecretDialogComponent },
    { default: ServerFormModalComponent },
  ] = await Promise.all([
    import('$lib/components/superadmin/radius/AssignmentFormModal.svelte'),
    import('$lib/components/superadmin/radius/MappingFormModal.svelte'),
    import('$lib/components/superadmin/radius/MappingSecretDialog.svelte'),
    import('$lib/components/superadmin/radius/ServerFormModal.svelte'),
  ]);

  return {
    AssignmentFormModalComponent,
    MappingFormModalComponent,
    MappingSecretDialogComponent,
    ServerFormModalComponent,
  };
});

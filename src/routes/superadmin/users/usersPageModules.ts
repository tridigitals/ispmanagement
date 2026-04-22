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

export type SuperadminUsersModalModules = {
  UserDetailsModalComponent: DeferredComponent;
  UserActionModalsComponent: DeferredComponent;
};

export const loadSuperadminUsersModalModules =
  createCachedLoader<SuperadminUsersModalModules>(async () => {
    const [{ default: UserDetailsModalComponent }, { default: UserActionModalsComponent }] =
      await Promise.all([
        import('$lib/components/superadmin/users/UserDetailsModal.svelte'),
        import('$lib/components/superadmin/users/UserActionModals.svelte'),
      ]);

    return {
      UserDetailsModalComponent,
      UserActionModalsComponent,
    };
  });

import type { Component } from 'svelte';

type DeferredComponent = Component<any>;

export type NetworkIncidentDetailDrawerModule = {
  IncidentDetailDrawerComponent: DeferredComponent;
};

export type NetworkIncidentSimulateDrawerModule = {
  IncidentSimulateDrawerComponent: DeferredComponent;
};

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

export const loadIncidentDetailDrawer = createCachedLoader(async () => {
  const { default: IncidentDetailDrawerComponent } = await import(
    '$lib/components/network/IncidentDetailDrawer.svelte'
  );

  return {
    IncidentDetailDrawerComponent,
  };
});

export const loadIncidentSimulateDrawer = createCachedLoader(async () => {
  const { default: IncidentSimulateDrawerComponent } = await import(
    '$lib/components/network/IncidentSimulateDrawer.svelte'
  );

  return {
    IncidentSimulateDrawerComponent,
  };
});

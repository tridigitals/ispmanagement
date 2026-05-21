import { writable } from 'svelte/store';
import type { GlobalSearchResultGroup } from '$lib/search/globalSearchModel';

export interface GlobalSearchState {
  open: boolean;
  query: string;
  loading: boolean;
  groups: GlobalSearchResultGroup[];
}

const initialState: GlobalSearchState = {
  open: false,
  query: '',
  loading: false,
  groups: [],
};

export function createGlobalSearchStore() {
  const { subscribe, update, set } = writable<GlobalSearchState>(initialState);

  return {
    subscribe,
    open: () => update((state) => ({ ...state, open: true })),
    close: () => set(initialState),
    setQuery: (query: string) => update((state) => ({ ...state, query })),
    setLoading: (loading: boolean) => update((state) => ({ ...state, loading })),
    setResults: (groups: GlobalSearchResultGroup[]) => update((state) => ({ ...state, groups })),
  };
}

export const globalSearch = createGlobalSearchStore();

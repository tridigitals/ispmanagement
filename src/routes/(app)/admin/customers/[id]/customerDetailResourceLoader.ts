export type CustomerDetailResourceLoadResult<T> =
  | { status: 'cached' }
  | { status: 'loaded'; value: T };

type LoadOptions = {
  force?: boolean;
};

type InFlightEntry<T> = {
  key: string;
  requestId: number;
  promise: Promise<CustomerDetailResourceLoadResult<T>>;
};

export function createCustomerDetailResourceLoader<T>() {
  let loadedKey: string | null = null;
  let inFlight: InFlightEntry<T> | null = null;
  let lastRequestId = 0;

  return {
    hasLoaded(key: string): boolean {
      return loadedKey === key;
    },

    invalidate(key?: string) {
      if (!key || loadedKey === key) {
        loadedKey = null;
      }
      if (!key || inFlight?.key === key) {
        inFlight = null;
      }
    },

    async load(
      key: string,
      fetcher: () => Promise<T>,
      options: LoadOptions = {},
    ): Promise<CustomerDetailResourceLoadResult<T>> {
      if (!options.force && loadedKey === key) {
        return { status: 'cached' };
      }

      if (inFlight?.key === key) {
        return inFlight.promise;
      }

      const requestId = ++lastRequestId;
      const promise = (async () => {
        try {
          const value = await fetcher();
          loadedKey = key;
          return { status: 'loaded', value } as const;
        } finally {
          if (inFlight?.key === key && inFlight.requestId === requestId) {
            inFlight = null;
          }
        }
      })();

      inFlight = { key, requestId, promise };
      return promise;
    },
  };
}

import { describe, expect, it, vi } from 'vitest';

import { createCustomerDetailResourceLoader } from './customerDetailResourceLoader';

describe('customer detail resource loader', () => {
  it('dedupes concurrent requests for the same key', async () => {
    const loader = createCustomerDetailResourceLoader<number>();
    const fetcher = vi.fn(async () => {
      await Promise.resolve();
      return 42;
    });

    const [first, second] = await Promise.all([
      loader.load('cust-1', fetcher),
      loader.load('cust-1', fetcher),
    ]);

    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(first).toEqual({ status: 'loaded', value: 42 });
    expect(second).toEqual({ status: 'loaded', value: 42 });
  });

  it('skips reloading an already loaded key unless forced', async () => {
    const loader = createCustomerDetailResourceLoader<number>();
    const fetcher = vi.fn(async () => 7);

    const first = await loader.load('cust-1', fetcher);
    const second = await loader.load('cust-1', fetcher);
    const third = await loader.load('cust-1', fetcher, { force: true });

    expect(first).toEqual({ status: 'loaded', value: 7 });
    expect(second).toEqual({ status: 'cached' });
    expect(third).toEqual({ status: 'loaded', value: 7 });
    expect(fetcher).toHaveBeenCalledTimes(2);
  });

  it('invalidates a loaded key so the next request fetches again', async () => {
    const loader = createCustomerDetailResourceLoader<number>();
    const fetcher = vi.fn(async () => 9);

    await loader.load('cust-1', fetcher);
    loader.invalidate('cust-1');
    const result = await loader.load('cust-1', fetcher);

    expect(result).toEqual({ status: 'loaded', value: 9 });
    expect(fetcher).toHaveBeenCalledTimes(2);
  });

  it('starts a fresh forced request when an older request is still in flight', async () => {
    const loader = createCustomerDetailResourceLoader<number>();
    let resolveFirst!: (value: number) => void;
    const first = new Promise<number>((resolve) => {
      resolveFirst = resolve;
    });
    const fetcher = vi
      .fn<() => Promise<number>>()
      .mockReturnValueOnce(first)
      .mockResolvedValueOnce(2);

    const staleRequest = loader.load('cust-1', fetcher);
    const freshRequest = loader.load('cust-1', fetcher, { force: true });
    resolveFirst(1);

    await expect(freshRequest).resolves.toEqual({ status: 'loaded', value: 2 });
    await expect(staleRequest).resolves.toEqual({ status: 'loaded', value: 1 });
    expect(await loader.load('cust-1', fetcher)).toEqual({ status: 'cached' });
    expect(fetcher).toHaveBeenCalledTimes(2);
  });
});

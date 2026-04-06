import { describe, expect, it, vi } from 'vitest';

import { createThenApplyPppoeAccount } from './pppoeCreateProvisioning';

describe('createThenApplyPppoeAccount', () => {
  it('creates first and then applies the created PPPoE account', async () => {
    const events: string[] = [];
    const created = { id: 'pppoe-1' };
    const create = vi.fn(async () => {
      events.push('create');
      return created;
    });
    const apply = vi.fn(async (id: string) => {
      events.push(`apply:${id}`);
    });

    const result = await createThenApplyPppoeAccount({ create, apply });

    expect(create).toHaveBeenCalledTimes(1);
    expect(apply).toHaveBeenCalledWith('pppoe-1');
    expect(events).toEqual(['create', 'apply:pppoe-1']);
    expect(result).toEqual({
      created,
      applyAttempted: true,
      applySucceeded: true,
    });
  });

  it('returns a non-applied result when the created row has no id', async () => {
    const create = vi.fn(async () => ({ username: 'demo' }));
    const apply = vi.fn();

    const result = await createThenApplyPppoeAccount({ create, apply });

    expect(apply).not.toHaveBeenCalled();
    expect(result).toEqual({
      created: { username: 'demo' },
      applyAttempted: false,
      applySucceeded: false,
    });
  });

  it('keeps the created row when apply fails so the caller can surface a partial success', async () => {
    const created = { id: 'pppoe-2' };
    const create = vi.fn(async () => created);
    const applyError = new Error('router unavailable');
    const apply = vi.fn(async () => {
      throw applyError;
    });

    await expect(createThenApplyPppoeAccount({ create, apply })).rejects.toMatchObject({
      created,
      applyError,
    });
  });
});

export type PppoeCreateResult = {
  id?: string | null;
};

export type PppoeCreateThenApplySuccess<TCreated> = {
  created: TCreated;
  applyAttempted: boolean;
  applySucceeded: boolean;
};

export class PppoeCreateApplyError<TCreated> extends Error {
  created: TCreated;
  applyError: unknown;

  constructor(created: TCreated, applyError: unknown) {
    super('PPPoE account was created, but apply failed');
    this.name = 'PppoeCreateApplyError';
    this.created = created;
    this.applyError = applyError;
  }
}

export async function createThenApplyPppoeAccount<TCreated>({
  create,
  apply,
}: {
  create: () => Promise<TCreated>;
  apply: (id: string) => Promise<unknown>;
}): Promise<PppoeCreateThenApplySuccess<TCreated>> {
  const created = await create();
  const createdId =
    created && typeof created === 'object' && 'id' in created
      ? String((created as PppoeCreateResult).id ?? '').trim()
      : '';

  if (!createdId) {
    return {
      created,
      applyAttempted: false,
      applySucceeded: false,
    };
  }

  try {
    await apply(createdId);
  } catch (applyError) {
    throw new PppoeCreateApplyError(created, applyError);
  }

  return {
    created,
    applyAttempted: true,
    applySucceeded: true,
  };
}

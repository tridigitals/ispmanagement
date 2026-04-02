import { describe, expect, it } from 'vitest';

import { canCopyManagedRadiusSecret, getManagedRadiusDisplayedSecret } from './managedRadiusSetup';

describe('managed radius setup helpers', () => {
  it('shows masked secret by default', () => {
    expect(
      getManagedRadiusDisplayedSecret(
        {
          shared_secret: 'secret-clear',
          shared_secret_masked: 'secr••••••••lear',
        },
        false,
      ),
    ).toBe('secr••••••••lear');
  });

  it('shows clear secret when reveal is enabled', () => {
    expect(
      getManagedRadiusDisplayedSecret(
        {
          shared_secret: 'secret-clear',
          shared_secret_masked: 'secr••••••••lear',
        },
        true,
      ),
    ).toBe('secret-clear');
  });

  it('returns placeholder when secret is missing', () => {
    expect(
      getManagedRadiusDisplayedSecret(
        {
          shared_secret: null,
          shared_secret_masked: null,
        },
        true,
      ),
    ).toBe('—');
  });

  it('only allows copy when clear secret exists', () => {
    expect(canCopyManagedRadiusSecret({ shared_secret: 'abc' })).toBe(true);
    expect(canCopyManagedRadiusSecret({ shared_secret: null })).toBe(false);
  });
});

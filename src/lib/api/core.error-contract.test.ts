import { describe, expect, it } from 'vitest';
import { extractApiErrorCode } from './core';

describe('API error contract', () => {
  it('extracts structured plan feature code from backend message', () => {
    expect(
      extractApiErrorCode({
        message: 'PLAN_FEATURE_REQUIRED:audit_logs: Upgrade your plan to access Audit Logs.',
      }),
    ).toBe('PLAN_FEATURE_REQUIRED');
  });

  it('does not classify ordinary forbidden errors as plan entitlement errors', () => {
    expect(extractApiErrorCode({ message: 'Missing permission audit_logs:read' })).toBeUndefined();
  });
});

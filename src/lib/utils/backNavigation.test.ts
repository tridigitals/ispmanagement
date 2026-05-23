import { describe, expect, it } from 'vitest';

import { appendBackParam, resolveBackTarget } from './backNavigation';

describe('back navigation helpers', () => {
  it('appends the current internal route as a back param', () => {
    const currentUrl = new URL('https://demo.test/admin/network/map?focus=customer-1');

    expect(appendBackParam('/admin/customers/cust-1', currentUrl)).toBe(
      '/admin/customers/cust-1?back=%2Fadmin%2Fnetwork%2Fmap%3Ffocus%3Dcustomer-1',
    );
  });

  it('preserves existing target query params when appending back', () => {
    const currentUrl = new URL('https://demo.test/admin/network/map?focus=service-99');

    expect(
      appendBackParam('/admin/customers/cust-1?tab=subscriptions&service_id=svc-99', currentUrl),
    ).toBe(
      '/admin/customers/cust-1?tab=subscriptions&service_id=svc-99&back=%2Fadmin%2Fnetwork%2Fmap%3Ffocus%3Dservice-99',
    );
  });

  it('resolves a valid internal back target from the current page url', () => {
    const pageUrl = new URL(
      'https://demo.test/admin/customers/cust-1?back=%2Fadmin%2Fnetwork%2Fmap%3Ffocus%3Dcustomer-1',
    );

    expect(resolveBackTarget(pageUrl, '/admin/customers')).toBe('/admin/network/map?focus=customer-1');
  });

  it('falls back when the back target is missing', () => {
    const pageUrl = new URL('https://demo.test/admin/customers/cust-1');

    expect(resolveBackTarget(pageUrl, '/admin/customers')).toBe('/admin/customers');
  });

  it('falls back when the back target is external or protocol-relative', () => {
    const externalUrl = new URL(
      'https://demo.test/admin/customers/cust-1?back=https%3A%2F%2Fevil.example%2Fsteal',
    );
    const protocolRelativeUrl = new URL(
      'https://demo.test/admin/customers/cust-1?back=%2F%2Fevil.example%2Fsteal',
    );

    expect(resolveBackTarget(externalUrl, '/admin/customers')).toBe('/admin/customers');
    expect(resolveBackTarget(protocolRelativeUrl, '/admin/customers')).toBe('/admin/customers');
  });
});

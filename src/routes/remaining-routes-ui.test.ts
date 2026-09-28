import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('remaining route UI cleanup', () => {
  it('keeps remaining user-facing routes free of decorative gradients and hardcoded white panels', () => {
    const files = [
      'src/routes/+error.svelte',
      'src/routes/pay/[id]/+page.svelte',
      'src/routes/(v2)/v2/support/+page.svelte',
      'src/routes/(v2)/v2/dashboard/locations/+page.svelte',
      'src/routes/(v2)/v2/dashboard/services/+page.svelte',
      'src/routes/(v2)/v2/dashboard/services/order/+page.svelte',
      'src/routes/(v2)/v2/dashboard/services/order/internet/+page.svelte',
      'src/lib/components/settings/SettingsEmailTab.svelte',
      'src/lib/components/settings/SettingsPaymentTab.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toMatch(/(?:linear|radial)-gradient/);
      expect(source, file).not.toContain('backdrop-filter');
    }
  });
});

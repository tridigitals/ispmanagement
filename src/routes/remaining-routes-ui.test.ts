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
      'src/routes/(app)/support/+page.svelte',
      'src/routes/(app)/dashboard/locations/+page.svelte',
      'src/routes/(app)/dashboard/services/+page.svelte',
      'src/routes/(app)/dashboard/services/order/+page.svelte',
      'src/routes/(app)/dashboard/services/order/internet/+page.svelte',
      'src/routes/(app)/admin/services/ServicesDialogs.svelte',
      'src/routes/(app)/admin/settings/SettingsEmailTab.svelte',
      'src/routes/(app)/admin/settings/SettingsPaymentTab.svelte',
      'src/routes/(app)/notifications/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toMatch(/(?:linear|radial)-gradient/);
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toMatch(/background:\s*#(?:fff|ffffff)\b/i);
      expect(source, file).toContain('var(--bg-surface)');
    }
  });
});

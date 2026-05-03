import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('shared common component UI cleanup', () => {
  it('keeps shared non-map components free of decorative gradients, blur glass, and large hardcoded radii', () => {
    const files = [
      'src/lib/components/ui/Lightbox.svelte',
      'src/lib/components/ui/FileManager.svelte',
      'src/lib/components/ui/PdfViewer.svelte',
      'src/lib/components/ui/Select.svelte',
      'src/lib/components/ui/Select2.svelte',
      'src/lib/components/ui/Toggle.svelte',
      'src/lib/components/ui/Modal.svelte',
      'src/lib/components/ui/MobileFabMenu.svelte',
      'src/lib/components/profile/ProfileGeneralTab.svelte',
      'src/lib/components/profile/ProfileNotificationsTab.svelte',
      'src/lib/components/announcements/AnnouncementDetailView.svelte',
      'src/lib/components/layout/AnnouncementBanner.svelte',
      'src/lib/components/superadmin/users/UserDetailsModal.svelte',
      'src/lib/components/superadmin/users/UserTable.svelte',
      'src/lib/components/superadmin/system/SystemStatusBanner.svelte',
      'src/lib/components/superadmin/tenants/TenantTable.svelte',
      'src/lib/components/billing/TenantBillingPlanPanel.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toMatch(/(?:linear|radial)-gradient/);
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toMatch(/background:\s*#(?:fff|ffffff)\b/i);
      expect(source, file).not.toMatch(/border-radius:\s*(?:1[6-9]|2[0-9]|3[0-9])px/);
    }
  });
});

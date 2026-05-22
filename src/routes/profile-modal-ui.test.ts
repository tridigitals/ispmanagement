import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('profile modal integration', () => {
  it('opens profile from shared shell surfaces instead of navigating to a profile page', () => {
    const files = [
      'src/routes/(app)/+layout.svelte',
      'src/lib/components/layout/NotificationDropdown.svelte',
      'src/routes/(app)/dashboard/+page.svelte',
      'src/routes/(app)/notifications/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);
      expect(source, file).toContain('openProfileModal');
      expect(source, file).not.toContain('/profile?tab=');
    }
  });

  it('renders a dedicated reusable profile modal component and removes the profile route page', () => {
    const modalSource = readSource('src/lib/components/profile/ProfileModal.svelte');

    expect(modalSource).toContain('role="dialog"');
    expect(modalSource).toContain('profile-modal-backdrop');
    expect(modalSource).toContain('<ProfileSurface');
    expect(() => readSource('src/routes/(app)/profile/+page.svelte')).toThrow();
  });

  it('lets the profile surface change tabs locally after the modal seeds the initial tab', () => {
    const source = readSource('src/lib/components/profile/ProfileSurface.svelte');

    expect(source).toContain('let lastRequestedTab = $state<ProfileTabId | null>(null);');
    expect(source).toContain('if (!isProfileTabId(requestedTab) || requestedTab === lastRequestedTab) return;');
    expect(source).not.toContain("if (isProfileTabId(requestedTab) && requestedTab !== activeTab)");
  });
});

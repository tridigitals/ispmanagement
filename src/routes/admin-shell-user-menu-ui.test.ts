import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('shell user menu composition', () => {
  it('extracts the user dropdown into a reusable layout component', () => {
    const file = 'src/lib/components/layout/UserMenuDropdown.svelte';

    expect(existsSync(resolve(process.cwd(), file))).toBe(true);

    const source = readSource(file);
    expect(source).toContain("import { openProfileModal } from '$lib/stores/profileModal'");
    expect(source).toContain("openProfileModal({ tab: 'general' })");
    expect(source).not.toContain("goto(`${tenantPrefix}/profile`)");
    expect(source).toContain('handleLogout');
    expect(source).toContain("$t('sidebar.profile')");
    expect(source).toContain("$t('sidebar.logout')");
    expect(source).toContain('dropdown-menu');
    expect(source).toContain('top: calc(100% + 8px);');
    expect(source).not.toContain('bottom: calc(100% + 8px);');
    expect(source).toContain('handleWindowClick(event: MouseEvent)');
    expect(source).toContain('rootEl?.contains(event.target)');
  });

  it('renders the reusable user menu in the desktop topbar beside notifications', () => {
    const topbar = readSource('src/lib/components/layout/Topbar.svelte');

    expect(topbar).toContain("import UserMenuDropdown from './UserMenuDropdown.svelte'");
    expect(topbar).toMatch(
      /<div class="right-section">[\s\S]*<NotificationDropdown \/>[\s\S]*<UserMenuDropdown variant="topbar" \/>[\s\S]*<\/div>/,
    );
    expect(topbar).toContain('topbar-user-menu');
    expect(topbar).toMatch(/@media \(max-width:\s*900px\)[\s\S]*\.topbar-user-menu\s*\{[\s\S]*display:\s*none/);
  });

  it('keeps the profile section in the sidebar only for mobile drawer usage', () => {
    const sidebar = readSource('src/lib/components/layout/Sidebar.svelte');

    expect(sidebar).toContain("import UserMenuDropdown from './UserMenuDropdown.svelte'");
    expect(sidebar).toContain('mobile-profile-section');
    expect(sidebar).toContain('<UserMenuDropdown variant="sidebar"');
    expect(sidebar).not.toContain('class="profile-btn"');
    expect(sidebar).not.toContain('class="dropdown-menu"');
    expect(sidebar).toMatch(
      /@media \(min-width:\s*900px\)[\s\S]*\.mobile-profile-section\s*\{[\s\S]*display:\s*none/,
    );
  });
});

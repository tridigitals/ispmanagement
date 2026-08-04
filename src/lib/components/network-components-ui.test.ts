import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('network and shell component UI cleanup', () => {
  it('keeps network, wallboard, shell, and superadmin setting components on clean tokens', () => {
    const files = [
      'src/lib/components/layout/NotificationDropdown.svelte',
      'src/lib/components/layout/Topbar.svelte',
      'src/lib/components/layout/Sidebar.svelte',
      'src/lib/components/superadmin/settings/SettingsGeneralTab.svelte',
      'src/lib/components/superadmin/settings/SettingsPaymentTab.svelte',
      'src/lib/components/superadmin/settings/SettingsAlertingTab.svelte',
      'src/lib/components/superadmin/settings/SettingsPasswordTab.svelte',
      'src/lib/components/superadmin/settings/SettingsAuthTab.svelte',
      'src/lib/components/superadmin/settings/SettingsSecurityTab.svelte',
      'src/lib/components/superadmin/settings/SettingsBackupTab.svelte',
      'src/lib/components/network/WallboardSlotPicker.svelte',
      'src/lib/components/network/WallboardThresholdDialog.svelte',
      'src/lib/components/network/NetworkMapSearchBar.svelte',
      'src/lib/components/network/NetworkMapInsightStrip.svelte',
      'src/lib/components/network/NetworkMapSmartInspector.svelte',
      'src/lib/components/network/NetworkMapFloatingControls.svelte',
      'src/lib/components/network/NetworkMapLinkModal.svelte',
      'src/lib/components/network/WallboardFullDialog.svelte',
      'src/lib/components/network/WallboardInterfaceTile.svelte',
      'src/lib/components/network/MapCanvasShell.svelte',
      'src/lib/components/network/NetworkMapManager.svelte',
      'src/lib/components/network/NetworkMapQuickModes.svelte',
      'src/lib/components/network/mixradius/MixRadiusExecutionStep.svelte',
      'src/lib/components/network/mixradius/MixRadiusMappingStep.svelte',
      'src/lib/components/network/mixradius/MixRadiusUploadStep.svelte',
      'src/lib/components/network/mixradius/MixRadiusSourceSummaryStep.svelte',
      'src/lib/components/network/mixradius/MixRadiusImportWizard.svelte',
      'src/lib/components/network/mixradius/MixRadiusPreviewStep.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      if (!file.includes('/network/Wallboard')) {
        expect(source, file).not.toMatch(/(?:linear|radial)-gradient/);
        expect(source, file).not.toContain('backdrop-filter');
      }
      expect(source, file).not.toMatch(/background:\s*#(?:fff|ffffff)\b/i);
      expect(source, file).not.toMatch(/border-radius:\s*(?:1[6-9]|2[0-9]|3[0-9])px/);
    }
  });

  it('keeps the mobile app shell and topbar constrained to the viewport', () => {
    const topbar = readSource('src/lib/components/layout/Topbar.svelte');
    const layout = readSource('src/routes/(app)/+layout.svelte');
    const notifications = readSource('src/lib/components/layout/NotificationDropdown.svelte');

    expect(topbar).toContain('.topbar');
    expect(topbar).toMatch(/\.topbar\s*\{[\s\S]*min-width:\s*0/);
    expect(topbar).toMatch(/\.topbar\s*\{[\s\S]*overflow:\s*visible/);
    expect(topbar).toMatch(/\.left-section\s*\{[\s\S]*min-width:\s*0/);
    expect(topbar).toMatch(/\.center-section\s*\{[\s\S]*flex:\s*0 1 min\(40vw,\s*500px\)/);
    expect(topbar).toMatch(/\.page-title\s*\{[\s\S]*text-overflow:\s*ellipsis/);
    expect(topbar).toMatch(/\.page-title\s*\{[\s\S]*white-space:\s*nowrap/);
    expect(topbar).toMatch(/@media \(max-width:\s*900px\)[\s\S]*\.topbar\s*\{[\s\S]*overflow:\s*visible/);

    expect(layout).toMatch(/\.main-viewport\s*\{[\s\S]*min-width:\s*0/);
    expect(layout).toMatch(/\.content-surface\s*\{[\s\S]*min-width:\s*0/);
    expect(layout).toMatch(/\.scroll-area\s*\{[\s\S]*min-width:\s*0/);

    expect(notifications).toMatch(/@media \(max-width:\s*520px\)[\s\S]*max-width:\s*calc\(100dvw - 24px\)/);
    expect(notifications).toMatch(/@media \(max-width:\s*520px\)[\s\S]*box-sizing:\s*border-box/);
  });
});

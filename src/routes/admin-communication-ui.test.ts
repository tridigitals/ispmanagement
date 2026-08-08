import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin communication UI cleanup', () => {
  it('keeps announcements, roles, audit logs, and outbox pages on restrained surfaces', () => {
    const files = [
      'src/routes/(app)/announcements/+page.svelte',
      'src/routes/(app)/admin/announcements/+page.svelte',
      'src/routes/(app)/admin/audit-logs/+page.svelte',
      'src/routes/(app)/admin/email-outbox/+page.svelte',
      'src/routes/(app)/admin/roles/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toMatch(/(?:linear|radial)-gradient/);
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toMatch(/border-radius:\s*(?:1[6-9]|[2-9][0-9])px/);
      expect(source, file).not.toMatch(/background:\s*#(?:fff|ffffff)\b/i);
      expect(source, file).toContain('var(--bg-surface)');
    }
  });

  it('exposes a restrained responsive message template builder with RBAC guard', () => {
    const source = readSource('src/routes/(app)/admin/message-templates/+page.svelte');

    expect(source).toContain("canManageTemplates");
    expect(source).toContain("communication_templates");
    expect(source).toContain("messageTemplates.list");
    expect(source).toContain("messageTemplates.preview");
    expect(source).toContain("Variable");
    expect(source).toContain("@media (max-width: 720px)");
    expect(source).not.toMatch(/(?:linear|radial)-gradient/);
    expect(source).not.toContain('backdrop-filter');
  });

  it('keeps audit entitlement errors structured for upgrade state', () => {
    const source = readSource('src/routes/(app)/admin/audit-logs/+page.svelte');
    expect(source).toContain("errorCode === 'PLAN_FEATURE_REQUIRED'");
    expect(source).toContain("errorMessage.toLowerCase().includes('upgrade')");
  });
});

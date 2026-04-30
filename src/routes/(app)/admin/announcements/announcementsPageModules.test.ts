import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  richTextEditor: { name: 'announcement-rich-text-editor' },
}));

vi.mock('$lib/components/ui/RichTextEditor.svelte', () => ({
  default: sentinels.richTextEditor,
}));

import { loadAnnouncementEditorComponent } from './announcementsPageModules';

describe('announcements page modules', () => {
  it('loads and caches the announcement editor lazily', async () => {
    const first = await loadAnnouncementEditorComponent();
    const second = await loadAnnouncementEditorComponent();

    expect(first.EditorComponent).toBe(sentinels.richTextEditor);
    expect(second).toBe(first);
  });
});

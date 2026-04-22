import type { Component } from 'svelte';

type DeferredComponent = Component<any>;
type AsyncModuleLoader<T> = () => Promise<T>;

export type AnnouncementEditorModule = {
  EditorComponent: DeferredComponent;
};

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export const loadAnnouncementEditorComponent = createCachedLoader(async () => {
  const { default: EditorComponent } = await import('$lib/components/ui/RichTextEditor.svelte');

  return {
    EditorComponent,
  };
});

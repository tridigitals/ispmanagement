type AsyncModuleLoader<T> = () => Promise<T>;

function createCachedLoader<T>(loader: AsyncModuleLoader<T>): AsyncModuleLoader<T> {
  let cached: Promise<T> | null = null;

  return () => {
    if (!cached) {
      cached = loader();
    }
    return cached;
  };
}

export const loadPreviewViewerModules = createCachedLoader(async () => {
  const [{ default: PdfViewerComponent }, { default: OfficeViewerComponent }] = await Promise.all([
    import('$lib/components/ui/PdfViewer.svelte'),
    import('$lib/components/ui/OfficeViewer.svelte'),
  ]);

  return {
    PdfViewerComponent,
    OfficeViewerComponent,
  };
});

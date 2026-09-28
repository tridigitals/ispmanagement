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

export const loadCollectionExportModule = createCachedLoader(async () => {
  const module = await import('$lib/utils/tabularExport');

  return {
    exportCsvRows: module.exportCsvRows,
    exportExcelRows: module.exportExcelRows,
  };
});

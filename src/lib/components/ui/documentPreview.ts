export type OfficePreviewKind = 'docx' | 'xlsx' | 'pptx' | null;

export function getOfficePreviewKind(file: {
  original_name?: string | null;
  content_type?: string | null;
}): OfficePreviewKind {
  const ext = (file?.original_name || '').split('.').pop()?.toLowerCase() || '';
  const contentType = file?.content_type || '';

  if (
    ext === 'docx' ||
    contentType === 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
  ) {
    return 'docx';
  }

  if (
    ext === 'xlsx' ||
    contentType === 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
  ) {
    return 'xlsx';
  }

  if (
    ext === 'pptx' ||
    contentType === 'application/vnd.openxmlformats-officedocument.presentationml.presentation'
  ) {
    return 'pptx';
  }

  return null;
}

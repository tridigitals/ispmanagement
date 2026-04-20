import { describe, expect, it } from 'vitest';

import { getOfficePreviewKind } from './documentPreview';

describe('document preview kind', () => {
  it('detects docx, xlsx, and pptx files', () => {
    expect(getOfficePreviewKind({ original_name: 'proposal.docx' })).toBe('docx');
    expect(getOfficePreviewKind({ original_name: 'finance.xlsx' })).toBe('xlsx');
    expect(getOfficePreviewKind({ original_name: 'deck.pptx' })).toBe('pptx');
  });

  it('detects office files from content types', () => {
    expect(
      getOfficePreviewKind({
        original_name: 'unknown.bin',
        content_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      }),
    ).toBe('docx');
  });

  it('returns null for unsupported files', () => {
    expect(getOfficePreviewKind({ original_name: 'notes.txt', content_type: 'text/plain' })).toBe(
      null,
    );
  });
});

import { jsPDF } from 'jspdf';
import html2canvas from 'html2canvas-pro';

/**
 * Generate a PDF from an HTML element and trigger a browser download.
 *
 * The element is rendered to a high-DPI canvas via html2canvas-pro, then
 * embedded into an A4 jsPDF document. Multi-page support: if the rendered
 * canvas is taller than one A4 page it is split across pages automatically.
 *
 * @param element  The DOM node to capture (e.g. the invoice-print-area div).
 * @param filename Download filename, e.g. "Invoice-INV-001.pdf".
 */
export async function generateInvoicePdf(element: HTMLElement, filename: string): Promise<void> {
  // 1. Render element to canvas at 2× resolution for crisp text
  const canvas = await html2canvas(element, {
    scale: 2,
    useCORS: true,
    backgroundColor: '#ffffff',
    logging: false,
    // html2canvas-pro supports this to ignore certain elements
    ignoreElements: (el) => el.classList.contains('invoice-modal-toolbar'),
  });

  const imgData = canvas.toDataURL('image/png');

  // 2. Create A4 PDF (portrait, mm)
  const pdf = new jsPDF('p', 'mm', 'a4');
  const pageWidth = pdf.internal.pageSize.getWidth();   // 210
  const pageHeight = pdf.internal.pageSize.getHeight();  // 297

  // Fit image width to page, preserve aspect ratio
  const imgWidth = pageWidth;
  const imgHeight = (canvas.height * imgWidth) / canvas.width;

  // 3. Split across pages if needed
  let yOffset = 0;
  let remainingHeight = imgHeight;

  while (remainingHeight > 0) {
    const sliceHeight = Math.min(remainingHeight, pageHeight);

    // Add the image slice — the y offset inside the source canvas maps to
    // a negative Y shift so only the correct portion is visible on each page.
    pdf.addImage(
      imgData,
      'PNG',
      0,                          // x
      -yOffset,                   // y (shift up to show the right slice)
      imgWidth,                   // rendered width
      imgHeight,                  // full height (cropped by page boundary)
    );

    remainingHeight -= sliceHeight;
    yOffset += sliceHeight;

    if (remainingHeight > 0) {
      pdf.addPage();
    }
  }

  // 4. Trigger download
  pdf.save(filename);
}

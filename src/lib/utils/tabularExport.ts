export type TabularRow = Record<string, unknown>;

function csvEscape(value: unknown) {
  const str = String(value ?? '');
  if (/[",\r\n]/.test(str)) return `"${str.replaceAll('"', '""')}"`;
  return str;
}

function htmlEscape(value: unknown) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function sanitizeFilePart(input: string) {
  return input
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-_]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 80);
}

export function buildTimestampedFilename(prefix: string, ext: 'csv' | 'xls') {
  const safePrefix = sanitizeFilePart(prefix) || 'export';
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  return `${safePrefix}-${stamp}.${ext}`;
}

export function triggerBlobDownload(filename: string, blob: Blob) {
  if (typeof document === 'undefined') return false;
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
  return true;
}

export function exportCsvRows(rows: TabularRow[], filenamePrefix: string) {
  if (!rows.length) return false;
  const headers = Object.keys(rows[0]);
  const lines = [headers.join(',')];
  for (const row of rows) {
    lines.push(headers.map((k) => csvEscape(row[k])).join(','));
  }
  const csv = lines.join('\n');
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  return triggerBlobDownload(buildTimestampedFilename(filenamePrefix, 'csv'), blob);
}

export function exportExcelRows(rows: TabularRow[], filenamePrefix: string) {
  if (!rows.length) return false;
  const headers = Object.keys(rows[0]);
  const headHtml = headers.map((h) => `<th>${htmlEscape(h)}</th>`).join('');
  const bodyHtml = rows
    .map((row) => `<tr>${headers.map((h) => `<td>${htmlEscape(row[h])}</td>`).join('')}</tr>`)
    .join('');
  const html = `<!doctype html><html><head><meta charset="utf-8" /></head><body><table border="1"><thead><tr>${headHtml}</tr></thead><tbody>${bodyHtml}</tbody></table></body></html>`;
  const blob = new Blob([html], { type: 'application/vnd.ms-excel;charset=utf-8;' });
  return triggerBlobDownload(buildTimestampedFilename(filenamePrefix, 'xls'), blob);
}

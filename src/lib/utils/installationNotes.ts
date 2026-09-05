/**
 * Helper murni untuk catatan WO Instalasi (gelombang 20).
 *
 * Halaman legacy (2.988 baris) menyimpan checklist instalasi DAN daftar foto
 * sebagai teks terformat di dalam kolom `notes` yang sama dengan catatan
 * manual teknisi — parse/strip-nya hidup di dalam komponen dan tak pernah
 * teruji. Bug nyata dari pola ini: checklist '[x] Cable installed' juga bisa
 * muncul di catatan manual teknisi (bukan dari checklist kita) dan strip
 * regex menelan baris milik user; atau foto dengan URL tanpa /content hilang.
 *
 * Fungsi di sini menyalin SEMANTIK legacy apa adanya (kompatibel baca-tulis
 * dengan data lama) tapi keluar dari komponen supaya bisa diuji.
 */

export interface InstallationChecklistState {
  cable: boolean;
  ont: boolean;
  pppoe: boolean;
  speed: boolean;
}

export const CHECKLIST_ITEMS: Array<{ key: keyof InstallationChecklistState; label: string }> = [
  { key: 'cable', label: 'Cable installed' },
  { key: 'ont', label: 'ONT installed' },
  { key: 'pppoe', label: 'PPPoE configured' },
  { key: 'speed', label: 'Speed test passed' },
];

/** Parse status checklist dari notes. Sama seperti legacy: regex per label,
 *  case-insensitive, mencari bentuk `[x] <label>`. */
export function parseChecklistState(notes: string | null | undefined): InstallationChecklistState {
  const raw = String(notes || '');
  const hasChecked = (label: string) => {
    const escaped = label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    return new RegExp(`\\[x\\]\\s+${escaped}`, 'i').test(raw);
  };
  return {
    cable: hasChecked('Cable installed'),
    ont: hasChecked('ONT installed'),
    pppoe: hasChecked('PPPoE configured'),
    speed: hasChecked('Speed test passed'),
  };
}

/** Bangun blok checklist dari state. Format identik dengan legacy agar data
 *  lama tetap terbaca dan baru tetap terbaca legacy. */
export function buildChecklistNote(state: InstallationChecklistState): string {
  const lines = CHECKLIST_ITEMS.map((it) => `${state[it.key] ? '[x]' : '[ ]'} ${it.label}`);
  return `Installation checklist:\n${lines.join('\n')}`;
}

/** Hapus blok generated (checklist & foto) dari notes, sisakan teks manual.
 *  Salinan persis regex legacy — termasuk batas 8 baris checklist. */
export function stripGeneratedSections(notes: string | null | undefined): string {
  const raw = String(notes || '').replace(/\r\n/g, '\n');
  return raw
    .replace(/(?:^|\n)Installation checklist:\n(?:\[[xX ]\]\s.*(?:\n|$)){1,8}/g, '\n')
    .replace(/(?:^|\n)Installation photos:\n(?:- .*(?:\n|$))+/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

/** Ekstrak file id foto dari URL /storage/files/<id>/content di notes. */
export function parsePhotoIds(notes: string | null | undefined): string[] {
  if (!notes) return [];
  const ids = new Set<string>();
  const regex = /\/storage\/files\/([0-9a-fA-F-]{8,})\/content/g;
  let match: RegExpExecArray | null = null;
  while ((match = regex.exec(notes)) !== null) {
    if (match[1]) ids.add(match[1]);
  }
  return Array.from(ids);
}

/** Susun notes final: teks manual + checklist + blok foto. Bagian kosong
 *  dilewati (sama seperti legacy join filter). */
export function buildPersistedNotes(
  manualNotes: string,
  checklist: InstallationChecklistState,
  photoLines: string[],
): string {
  const extra = stripGeneratedSections(manualNotes);
  const checklistBlock = buildChecklistNote(checklist);
  const photosBlock =
    photoLines.length === 0 ? '' : `Installation photos:\n${photoLines.join('\n')}`;
  return [extra, checklistBlock, photosBlock]
    .filter((part) => part && part.trim().length > 0)
    .join('\n\n');
}

/** Label status WO dalam bahasa Indonesia (dipakai badge & empty state). */
export function woStatusLabel(status: string): string {
  const map: Record<string, string> = {
    pending: 'Menunggu',
    in_progress: 'Dikerjakan',
    completed: 'Selesai',
    cancelled: 'Batal',
  };
  return map[status] ?? status;
}

/** Pesan error mentah backend -> kalimat yang bisa dibaca dispatcher. */
export function friendlyWorkOrderError(message: string | null | undefined): string {
  const raw = String(message || '');
  if (/already taken by another technician/i.test(raw)) {
    return 'WO ini baru saja diambil teknisi lain — muat ulang daftar.';
  }
  if (/Only pending work order can be taken/i.test(raw)) {
    return 'Hanya WO berstatus Menunggu yang bisa diambil.';
  }
  if (/Only admin\/owner can (cancel|reopen|release)/i.test(raw)) {
    return 'Hanya admin/owner yang boleh membatalkan, membuka ulang, atau melepas WO.';
  }
  if (/Cancellation reason is (required|too short)/i.test(raw)) {
    return 'Alasan pembatalan wajib diisi minimal 10 karakter.';
  }
  if (/Set installation schedule before starting/i.test(raw)) {
    return 'Isi jadwal instalasi sebelum memulai pengerjaan.';
  }
  if (/Set assignee before starting/i.test(raw)) {
    return 'Tentukan teknisi sebelum memulai pengerjaan.';
  }
  if (/Terminal asset must be selected/i.test(raw)) {
    return 'Pilih aset terminal (ONT/ONU) sebelum menyelesaikan instalasi.';
  }
  if (/Subscription status changed concurrently/i.test(raw)) {
    return 'Status langganan berubah bersamaan — coba selesaikan lagi.';
  }
  if (/No pending reschedule request/i.test(raw)) {
    return 'Tidak ada permintaan jadwal ulang yang tertunda untuk WO ini.';
  }
  return raw || 'Terjadi kesalahan tak terduga.';
}

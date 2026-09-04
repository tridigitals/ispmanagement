/**
 * Ringkasan tiket dukungan yang jujur.
 *
 * KENAPA INI ADA.
 *
 * Halaman lama (`(app)/admin/support/+page.svelte`) menampilkan lima kartu
 * ringkasan, dan dua di antaranya tidak bisa dipercaya:
 *
 *   baris  28: stats = { all, open, pending, closed }
 *   baris 262: <div class="stat-value">—</div>     <- kartu "Belum ditugaskan"
 *                                                    nilainya em-dash HARDCODE
 *
 * Backend (`http/support.rs`, `commands/support.rs`) hanya menghitung empat
 * angka: all, open, pending, closed. Tapi `resolve_support_ticket` menulis
 * `status = 'resolved'`, dan tidak ada SUM untuk status itu. Jadi setiap tiket
 * yang sudah diselesaikan hilang dari semua ember.
 *
 * Terukur di tenant "ISP Management" (probe-support-stale.cjs):
 *
 *   Total 20 · Open 18 · Pending 0 · Closed 1 · Belum ditugaskan "—"
 *   18 + 0 + 1 = 19, selisih 1 tiket tidak masuk ember mana pun
 *   DB: 9 tiket belum ditugaskan (7 masih aktif)
 *
 * Bug ketiga: `normalize_status()` hanya menerima open/pending/closed, jadi
 * filter `status=resolved` jatuh ke `None` — permintaan "tampilkan yang
 * resolved" balas SEMUA tiket tanpa pesan galat. Dan `pending` sendiri tidak
 * pernah ditulis backend mana pun, jadi kartunya selamanya 0.
 *
 * Modul ini menurunkan angka ringkasan dari satu tempat supaya aturannya bisa
 * diuji tanpa merender halaman, dan supaya penambahan status berikutnya tidak
 * lagi diam-diam hilang dari ringkasan.
 */

/** Status yang benar-benar ditulis backend, urut alur kerja. */
export const TICKET_STATUSES = ['open', 'pending', 'resolved', 'closed'] as const;
export type TicketStatus = (typeof TICKET_STATUSES)[number];

/** Status yang dianggap masih butuh pekerjaan. */
export const ACTIVE_STATUSES: readonly TicketStatus[] = ['open', 'pending'];

export interface StatsLike {
  all?: number | null;
  open?: number | null;
  pending?: number | null;
  closed?: number | null;
  resolved?: number | null;
  unassigned?: number | null;
}

export interface StatCard {
  key: 'all' | TicketStatus | 'unassigned';
  label: string;
  value: number;
  /** Konteks wajib — StatTile menolak angka tanpa penjelasan. */
  hint: string;
  tone: 'neutral' | 'positive' | 'negative' | 'warning';
  /** Filter status yang dipasang kartu ini; null untuk 'semua'. */
  filter: TicketStatus | 'unassigned' | null;
}

const n = (v: number | null | undefined) => (typeof v === 'number' && Number.isFinite(v) ? v : 0);

function persen(bagian: number, total: number): string {
  if (total <= 0) return '0%';
  return `${Math.round((bagian / total) * 1000) / 10}%`.replace('.', ',');
}

/**
 * Selisih antara total dan jumlah ember yang diketahui.
 *
 * Ini jaring pengaman permanen: kalau nanti ada status baru yang ditulis
 * backend tapi belum masuk ringkasan, angkanya muncul di sini alih-alih hilang
 * diam-diam seperti 'resolved' dulu.
 */
export function unaccounted(stats: StatsLike): number {
  const berember = n(stats.open) + n(stats.pending) + n(stats.resolved) + n(stats.closed);
  return Math.max(0, n(stats.all) - berember);
}

export function buildStatCards(stats: StatsLike): StatCard[] {
  const total = n(stats.all);
  const open = n(stats.open);
  const pending = n(stats.pending);
  const resolved = n(stats.resolved);
  const closed = n(stats.closed);
  const unassigned = n(stats.unassigned);
  const aktif = open + pending;

  return [
    {
      key: 'all',
      label: 'Semua tiket',
      value: total,
      hint: `${aktif} masih aktif · ${resolved + closed} selesai`,
      tone: 'neutral',
      filter: null,
    },
    {
      key: 'open',
      label: 'Terbuka',
      value: open,
      hint: total ? `${persen(open, total)} dari seluruh tiket` : 'belum ada tiket',
      tone: open > 0 ? 'warning' : 'neutral',
      filter: 'open',
    },
    {
      key: 'unassigned',
      label: 'Belum ditugaskan',
      value: unassigned,
      hint: aktif ? `dari ${aktif} tiket aktif` : 'tidak ada tiket aktif',
      tone: unassigned > 0 ? 'negative' : 'positive',
      filter: 'unassigned',
    },
    {
      key: 'resolved',
      label: 'Diselesaikan',
      value: resolved,
      hint: total ? `${persen(resolved, total)} dari seluruh tiket` : 'belum ada tiket',
      tone: 'positive',
      filter: 'resolved',
    },
    {
      key: 'closed',
      label: 'Ditutup',
      value: closed,
      hint: total ? `${persen(closed, total)} dari seluruh tiket` : 'belum ada tiket',
      tone: 'neutral',
      filter: 'closed',
    },
  ];
}

/**
 * Apakah kartu status ini layak ditampilkan?
 *
 * `pending` tidak pernah ditulis backend mana pun (dicek: tidak ada
 * `SET status = 'pending'` di seluruh `src-tauri/src`), jadi kartunya selalu 0
 * dan mengklik filternya selalu memberi tabel kosong. Menyembunyikannya saat
 * nol lebih jujur daripada memamerkan ember mati.
 */
export function shouldShowPending(stats: StatsLike): boolean {
  return n(stats.pending) > 0;
}

export interface AgeBucket {
  label: string;
  count: number;
}

/**
 * Umur tiket aktif — informasi yang sama sekali tidak ada di halaman lama.
 * Tiket open tertua di produksi berumur 195 hari; kolom "Updated" tidak
 * menunjukkan itu karena hanya menampilkan tanggal, bukan lama menunggu.
 */
export function bucketByAge(
  tickets: { status?: string | null; created_at?: string | null }[],
  now: number = Date.now(),
): AgeBucket[] {
  const HARI = 86_400_000;
  const b = { baru: 0, seminggu: 0, sebulan: 0, lama: 0 };

  for (const t of tickets) {
    if (!ACTIVE_STATUSES.includes((t.status ?? '') as TicketStatus)) continue;
    const ms = t.created_at ? now - new Date(t.created_at).getTime() : NaN;
    if (!Number.isFinite(ms)) continue;
    if (ms < 2 * HARI) b.baru++;
    else if (ms < 7 * HARI) b.seminggu++;
    else if (ms < 30 * HARI) b.sebulan++;
    else b.lama++;
  }

  return [
    { label: '< 2 hari', count: b.baru },
    { label: '2–7 hari', count: b.seminggu },
    { label: '1–4 minggu', count: b.sebulan },
    { label: '> 30 hari', count: b.lama },
  ];
}

/** Umur dalam kata, dipakai di kolom "Menunggu". */
export function waitingLabel(createdAt: string | null | undefined, now: number = Date.now()): string {
  if (!createdAt) return '—';
  const ms = now - new Date(createdAt).getTime();
  if (!Number.isFinite(ms) || ms < 0) return '—';
  const HARI = 86_400_000;
  if (ms < 3_600_000) return `${Math.max(1, Math.floor(ms / 60_000))} menit`;
  if (ms < HARI) return `${Math.floor(ms / 3_600_000)} jam`;
  if (ms < 30 * HARI) return `${Math.floor(ms / HARI)} hari`;
  return `${Math.floor(ms / (30 * HARI))} bulan`;
}

/** Tiket aktif yang menunggu lebih lama dari ambang ini disebut terlantar. */
export const STALE_TICKET_MS = 30 * 86_400_000;

export function isStale(
  t: { status?: string | null; created_at?: string | null },
  now: number = Date.now(),
): boolean {
  if (!ACTIVE_STATUSES.includes((t.status ?? '') as TicketStatus)) return false;
  if (!t.created_at) return false;
  const ms = now - new Date(t.created_at).getTime();
  return Number.isFinite(ms) && ms > STALE_TICKET_MS;
}

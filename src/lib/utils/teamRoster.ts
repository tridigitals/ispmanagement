/**
 * Aturan daftar anggota tim.
 *
 * KENAPA INI ADA.
 *
 * Halaman lama (`(app)/admin/team/+page.svelte`, 1.204 baris) menampilkan
 * pelanggan sebagai anggota tim. `team_service.rs:34 list_members()`
 * mengembalikan SEMUA baris `tenant_members` tanpa memfilter role, dan halaman
 * memakai hasilnya apa adanya.
 *
 * Terukur di tenant "ISP Management" (GET /api/team, 2026-09-04):
 *
 *   Owner 1 · Technician 1 · Member 1 · Customer 3   = 6 baris
 *   badge di layar: "6 members"
 *
 * Tiga di antaranya pelanggan: cobaorder@xtrabit.com, user1@xtrabit.com,
 * attacker@evil.com. Jadi angka "6 anggota tim" sebenarnya 3 staf + 3
 * pelanggan.
 *
 * Yang membuatnya lebih aneh: baris 98 halaman lama SUDAH memfilter Customer
 * dari dropdown filter role (`r.name.toLowerCase() !== 'customer'`), dan
 * backend menolak menetapkan role Customer lewat modul tim
 * (`http/team.rs:207` — "Create customer accounts from the Customers module
 * instead"). Jadi seluruh sistem sepakat Customer bukan anggota tim, kecuali
 * kueri yang mengisi tabelnya.
 *
 * Modul ini memisahkan staf dari pelanggan di satu tempat supaya aturannya
 * bisa diuji, dan supaya halaman lain yang memakai `api.team.list()`
 * (incidents, pencarian global) bisa ikut memakai aturan yang sama.
 */

export interface MemberLike {
  id?: string;
  user_id?: string;
  name?: string | null;
  email?: string | null;
  role?: string | null;
  role_name?: string | null;
  role_level?: number | null;
  is_active?: boolean;
  created_at?: string | null;
  deleted_at?: string | null;
  /**
   * null/undefined = kolom tidak di-SELECT endpoint itu, bukan "2FA mati".
   * Hanya `false` yang berarti akun benar-benar tanpa 2FA.
   */
  two_factor_enabled?: boolean | null;
  email_verified_at?: string | null;
}

/**
 * Nama role yang bukan staf.
 *
 * Dibandingkan case-insensitive terhadap `role_name` (dari tabel `roles`) dan
 * `role` (kolom teks lama di `tenant_members`, yang di produksi berisi
 * 'customer' huruf kecil untuk baris yang sama).
 */
const NON_STAFF = new Set(['customer']);

function roleWords(m: MemberLike): string[] {
  return [m.role_name, m.role].filter((x): x is string => typeof x === 'string' && x.length > 0);
}

export function isStaff(m: MemberLike): boolean {
  const words = roleWords(m);
  if (!words.length) return true; // tanpa role: tampilkan, jangan sembunyikan diam-diam
  return !words.some((w) => NON_STAFF.has(w.trim().toLowerCase()));
}

export function isCustomerAccount(m: MemberLike): boolean {
  return !isStaff(m);
}

/** Hanya staf. Ini yang seharusnya diisi ke tabel halaman Tim. */
export function staffOnly(members: MemberLike[]): MemberLike[] {
  return members.filter(isStaff);
}

export interface TeamSummary {
  /** Jumlah staf — angka yang benar untuk "anggota tim". */
  staff: number;
  /** Akun pelanggan yang ikut terbawa kueri. Bukan anggota tim. */
  customers: number;
  /** Total baris tenant_members apa adanya, dipakai untuk menjelaskan selisih. */
  rows: number;
  staffActive: number;
  staffInactive: number;
}

export function summarize(members: MemberLike[]): TeamSummary {
  const staff = members.filter(isStaff);
  return {
    staff: staff.length,
    customers: members.length - staff.length,
    rows: members.length,
    staffActive: staff.filter((m) => m.is_active !== false).length,
    staffInactive: staff.filter((m) => m.is_active === false).length,
  };
}

/**
 * Pemetaan role ke tone badge.
 *
 * Level dipakai kalau ada karena itu yang menentukan kewenangan sebenarnya;
 * nama role hanya label. Owner (100) dan Admin (50) ditandai berbeda supaya
 * akun berkuasa tidak menyamar sebagai anggota biasa.
 */
export function roleTone(m: MemberLike): 'positive' | 'warning' | 'info' | 'neutral' {
  const level = typeof m.role_level === 'number' ? m.role_level : null;
  if (level !== null) {
    if (level >= 100) return 'warning'; // Owner: kewenangan penuh
    if (level >= 50) return 'info'; // Admin
    if (level > 0) return 'neutral';
    return 'neutral';
  }
  const name = (m.role_name ?? m.role ?? '').toLowerCase();
  if (name === 'owner') return 'warning';
  if (name === 'admin') return 'info';
  return 'neutral';
}

/**
 * Apakah pengguna dengan level `myLevel` boleh mengubah anggota ini?
 *
 * Menyalin aturan backend (`enforce_member_role_change_permissions`,
 * `http/team.rs:15`): butuh level lebih TINGGI, bukan sama. Halaman lama
 * memakai aturan yang sama tapi menuliskannya inline dua kali di dalam markup
 * (baris 458 dan 467) dengan fallback `|| 0` yang berbeda letak, jadi mudah
 * lepas sinkron saat salah satu diubah.
 */
export function canManage(myLevel: number, target: MemberLike): boolean {
  const targetLevel = typeof target.role_level === 'number' ? target.role_level : 0;
  return myLevel > targetLevel;
}

/**
 * Peringatan keamanan tingkat akun.
 *
 * Data produksi: keenam akun punya `two_factor_enabled = false` dan
 * `email_verified_at = NULL`, termasuk Owner. Halaman lama tidak menampilkan
 * kolom apa pun soal ini, jadi tidak ada tempat yang memberi tahu bahwa akun
 * dengan kewenangan penuh hanya dilindungi kata sandi.
 */
export interface SecurityFlag {
  kind: 'no_2fa_privileged' | 'unverified_email' | 'inactive_staff';
  member: MemberLike;
  text: string;
}

/**
 * `two_factor_enabled` dan `email_verified_at` kini ikut dikirim
 * `team_service.rs list_members()` (ditambahkan bersama halaman ini). Keduanya
 * `Option`/nullable dengan arti yang dibedakan:
 *
 *   undefined atau null pada `two_factor_enabled` = endpoint tidak menyertakan
 *   kolomnya → JANGAN menuduh. Hanya `false` yang berarti 2FA benar-benar mati.
 *
 * Ini penting karena `TeamMemberWithUser` dipakai 5 kueri lain (assignment
 * tiket, servicer pelanggan) yang SELECT-nya lebih sempit.
 */
export function securityFlags(members: MemberLike[]): SecurityFlag[] {
  const out: SecurityFlag[] = [];
  for (const m of staffOnly(members)) {
    const level = typeof m.role_level === 'number' ? m.role_level : 0;

    if (level >= 50 && m.two_factor_enabled === false) {
      out.push({
        kind: 'no_2fa_privileged',
        member: m,
        text: `${m.name ?? m.email} (${m.role_name ?? m.role}) tidak memakai 2FA`,
      });
    }
    if ('email_verified_at' in m && m.email_verified_at === null) {
      out.push({
        kind: 'unverified_email',
        member: m,
        text: `${m.email} belum diverifikasi`,
      });
    }
    if (m.is_active === false) {
      out.push({
        kind: 'inactive_staff',
        member: m,
        text: `${m.name ?? m.email} nonaktif tapi masih terdaftar di tim`,
      });
    }
  }
  return out;
}

/** Inisial untuk avatar. Menangani nama satu kata dan spasi berlebih. */
export function initials(name: string | null | undefined): string {
  const parts = (name ?? '').trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return '?';
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

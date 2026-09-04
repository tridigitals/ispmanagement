import { describe, expect, it } from 'vitest';
import {
  canManage,
  initials,
  isCustomerAccount,
  isStaff,
  roleTone,
  securityFlags,
  staffOnly,
  summarize,
  type MemberLike,
} from './teamRoster';

/*
  Enam baris yang benar-benar dikembalikan GET /api/team untuk tenant
  "ISP Management" pada 2026-09-04. Kolom `role` (teks lama) memakai huruf
  kecil untuk sebagian baris — itu apa adanya di DB, bukan kesalahan tulis.
*/
const PROD: MemberLike[] = [
  { user_id: 'u1', name: 'X-Trabit', email: 'info@xtrabit.com', role: 'owner', role_name: 'Owner', role_level: 100, is_active: true },
  { user_id: 'u2', name: 'teknisi', email: 'teknisi@xtrabit.com', role: 'Technician', role_name: 'Technician', role_level: 20, is_active: true },
  { user_id: 'u3', name: 'asdsda', email: 'asdfasd@gmail.com', role: 'Member', role_name: 'Member', role_level: 10, is_active: true },
  { user_id: 'u4', name: 'Coba Order', email: 'cobaorder@xtrabit.com', role: 'customer', role_name: 'Customer', role_level: 0, is_active: true },
  { user_id: 'u5', name: 'User Pertama', email: 'attacker@evil.com', role: 'customer', role_name: 'Customer', role_level: 0, is_active: true },
  { user_id: 'u6', name: 'use1', email: 'user1@xtrabit.com', role: 'customer', role_name: 'Customer', role_level: 0, is_active: true },
];

describe('isStaff / isCustomerAccount', () => {
  it('memisahkan tiga pelanggan dari tiga staf', () => {
    expect(staffOnly(PROD).map((m) => m.email)).toEqual([
      'info@xtrabit.com',
      'teknisi@xtrabit.com',
      'asdfasd@gmail.com',
    ]);
    expect(PROD.filter(isCustomerAccount)).toHaveLength(3);
  });

  it('mengenali Customer dari role_name maupun kolom role huruf kecil', () => {
    expect(isStaff({ role_name: 'Customer' })).toBe(false);
    expect(isStaff({ role: 'customer' })).toBe(false);
    expect(isStaff({ role: 'CUSTOMER' })).toBe(false);
    expect(isStaff({ role_name: '  customer  ' })).toBe(false);
  });

  it('tidak salah menyaring role lain yang mengandung kata mirip', () => {
    expect(isStaff({ role_name: 'Customer Service', role_level: 25 })).toBe(true);
  });

  it('anggota tanpa role tetap ditampilkan, bukan disembunyikan', () => {
    expect(isStaff({ email: 'x@y.z' })).toBe(true);
    expect(isStaff({ role: null, role_name: null })).toBe(true);
  });
});

describe('summarize', () => {
  it('menghitung staf, bukan jumlah baris', () => {
    const s = summarize(PROD);
    expect(s.staff).toBe(3);
    expect(s.customers).toBe(3);
    expect(s.rows).toBe(6); // yang dipajang halaman lama sebagai "6 members"
  });

  it('aktif/nonaktif dihitung hanya untuk staf', () => {
    const s = summarize([
      ...PROD,
      { user_id: 'u7', name: 'Staf Nonaktif', role_name: 'Member', role_level: 10, is_active: false },
      { user_id: 'u8', name: 'Pelanggan Nonaktif', role_name: 'Customer', role_level: 0, is_active: false },
    ]);
    expect(s.staff).toBe(4);
    expect(s.staffActive).toBe(3);
    expect(s.staffInactive).toBe(1); // pelanggan nonaktif tidak ikut
  });

  it('daftar kosong aman', () => {
    expect(summarize([])).toEqual({
      staff: 0,
      customers: 0,
      rows: 0,
      staffActive: 0,
      staffInactive: 0,
    });
  });

  it('is_active tidak terdefinisi dianggap aktif', () => {
    expect(summarize([{ role_name: 'Member', role_level: 10 }]).staffActive).toBe(1);
  });
});

describe('roleTone', () => {
  it('menandai Owner dan Admin berbeda dari anggota biasa', () => {
    expect(roleTone({ role_level: 100 })).toBe('warning');
    expect(roleTone({ role_level: 50 })).toBe('info');
    expect(roleTone({ role_level: 20 })).toBe('neutral');
  });

  it('jatuh ke nama role kalau level tidak ada', () => {
    expect(roleTone({ role_name: 'Owner' })).toBe('warning');
    expect(roleTone({ role: 'admin' })).toBe('info');
    expect(roleTone({ role_name: 'Technician' })).toBe('neutral');
  });
});

describe('canManage', () => {
  it('butuh level lebih tinggi, bukan sama — sesuai aturan backend', () => {
    const owner = PROD[0];
    const teknisi = PROD[1];
    expect(canManage(100, teknisi)).toBe(true);
    expect(canManage(100, owner)).toBe(false); // Owner tidak bisa mengubah Owner
    expect(canManage(20, teknisi)).toBe(false);
    expect(canManage(20, PROD[2])).toBe(true); // Technician 20 > Member 10
  });

  it('role_level hilang dianggap 0', () => {
    expect(canManage(10, { role_name: 'Entah' })).toBe(true);
    expect(canManage(0, { role_name: 'Entah' })).toBe(false);
  });
});

describe('securityFlags', () => {
  /*
    Data users produksi (GET /api/users, 22 akun): keenam anggota tenant ini
    punya two_factor_enabled=false dan email_verified_at=null, termasuk Owner.
  */
  const withDetail = PROD.map((m) => ({
    ...m,
    two_factor_enabled: false,
    email_verified_at: null,
  }));

  it('menandai akun berkuasa tanpa 2FA', () => {
    const f = securityFlags(withDetail);
    const tfa = f.filter((x) => x.kind === 'no_2fa_privileged');
    expect(tfa).toHaveLength(1); // hanya Owner (level 100); Technician 20 tidak
    expect(tfa[0].text).toContain('X-Trabit');
    expect(tfa[0].text).toContain('Owner');
  });

  it('tidak menandai akun pelanggan sama sekali', () => {
    const f = securityFlags(withDetail);
    expect(f.some((x) => x.member.email === 'cobaorder@xtrabit.com')).toBe(false);
  });

  it('menandai email belum terverifikasi untuk staf', () => {
    const f = securityFlags(withDetail);
    expect(f.filter((x) => x.kind === 'unverified_email')).toHaveLength(3);
  });

  it('menandai staf nonaktif yang masih terdaftar', () => {
    const f = securityFlags([
      { user_id: 'z', name: 'Mantan', role_name: 'Member', role_level: 10, is_active: false },
    ]);
    expect(f.map((x) => x.kind)).toEqual(['inactive_staff']);
  });

  it('kolom yang tidak di-SELECT tidak dianggap pelanggaran', () => {
    // PROD tanpa field 2FA/verifikasi sama sekali — meniru endpoint sempit
    // seperti assignment tiket yang memakai struct yang sama.
    expect(securityFlags(PROD)).toEqual([]);
  });

  it('2FA aktif tidak ditandai, tapi email null tetap', () => {
    const f = securityFlags([
      {
        user_id: 'o',
        name: 'Owner Aman',
        email: 'o@x.id',
        role_name: 'Owner',
        role_level: 100,
        two_factor_enabled: true,
        email_verified_at: null,
      },
    ]);
    expect(f.map((x) => x.kind)).toEqual(['unverified_email']);
  });

  it('email sudah terverifikasi tidak ditandai', () => {
    const f = securityFlags([
      {
        user_id: 'o',
        email: 'o@x.id',
        role_name: 'Owner',
        role_level: 100,
        two_factor_enabled: true,
        email_verified_at: '2026-01-01T00:00:00Z',
      },
    ]);
    expect(f).toEqual([]);
  });
});

describe('initials', () => {
  it('menangani nama satu kata, dua kata, dan spasi berlebih', () => {
    expect(initials('X-Trabit')).toBe('X-');
    expect(initials('Coba Order')).toBe('CO');
    expect(initials('  User   Pertama  ')).toBe('UP');
  });

  it('nama kosong tidak menghasilkan avatar kosong', () => {
    expect(initials('')).toBe('?');
    expect(initials(null)).toBe('?');
    expect(initials(undefined)).toBe('?');
  });
});

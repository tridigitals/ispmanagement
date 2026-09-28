import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

/**
 * Bukti DEFINITIF tentang body HTTP yang dikirim `safeInvoke`.
 *
 * Sebelumnya body-nya saya TEBAK di skrip E2E dan salah: saya kira `memberId`
 * ikut terkirim. Ternyata `safeInvoke` membuang key yang cocok dengan
 * `:placeholder` rute. Menebak tidak cukup — di sini `fetch` di-mock dan body
 * yang benar-benar dikirim diperiksa.
 *
 * Latar bug: `UpdateMemberDto` memakai `#[serde(deny_unknown_fields)]`, jadi
 * key tak dikenal ditolak 422. `update_team_member_role` dulu mengirim `id`
 * (nama berbeda dari placeholder `:memberId`), sehingga lolos ke body dan
 * setiap Simpan gagal:
 *
 *   422 ... id: unknown field `id`, expected one of
 *   `roleId`, `role_id`, `isActive`, `is_active`, `emailVerified`, `email_verified`
 */
describe('safeInvoke — body yang benar-benar dikirim untuk PUT /team/:memberId', () => {
  let sent: { url: string; method: string; body: unknown } | null = null;

  beforeEach(() => {
    sent = null;

    const store = new Map<string, string>();
    store.set('auth_token', 'test-token');

    vi.stubGlobal('window', {
      ...globalThis,
      location: {
        protocol: 'http:',
        origin: 'http://localhost:1420',
        href: 'http://localhost:1420/',
      },
      addEventListener: () => {},
      removeEventListener: () => {},
    });
    vi.stubGlobal('localStorage', {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
      clear: () => store.clear(),
      key: () => null,
      get length() {
        return store.size;
      },
    });

    vi.stubGlobal('fetch', async (url: string, init: RequestInit) => {
      sent = {
        url: String(url),
        method: String(init.method),
        body: init.body ? JSON.parse(String(init.body)) : null,
      };
      return new Response(JSON.stringify({ success: true }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      });
    });
  });

  afterEach(() => vi.unstubAllGlobals());

  it('membuang `memberId` (placeholder) dan `id` dari body', async () => {
    const { team } = await import('./team');

    await team.update('MEMBER-123', {
      roleId: 'ROLE-9',
      isActive: false,
      emailVerified: true,
    });

    expect(sent, 'fetch harus dipanggil').not.toBeNull();
    expect(sent!.url, 'placeholder terisi di path').toContain('/team/MEMBER-123');
    expect(sent!.method).toBe('PUT');

    const body = sent!.body as Record<string, unknown>;
    // Inti regresi: dua key ini dulu ikut terkirim -> 422.
    expect(body, 'memberId tidak boleh ada di body').not.toHaveProperty('memberId');
    expect(body, 'id tidak boleh ada di body').not.toHaveProperty('id');
    // Yang harus ada (snake_case dipilih otomatis oleh safeInvoke):
    expect(body.roleId).toBe('ROLE-9');
    expect(body.is_active).toBe(false);
    expect(body.email_verified).toBe(true);
  });

  it('updateRole juga tidak mengirim `id` ke body', async () => {
    const { team } = await import('./team');
    await team.updateRole('MEMBER-9', 'ROLE-1');
    const body = sent!.body as Record<string, unknown>;
    expect(body).not.toHaveProperty('id');
    expect(body).not.toHaveProperty('memberId');
    expect(body.roleId).toBe('ROLE-1');
    expect(sent!.url).toContain('/team/MEMBER-9');
  });

  it('GET mengirim lewat query, bukan body', async () => {
    const { team } = await import('./team');
    await team.list();
    expect(sent!.url).toContain('/team');
    expect(sent!.method).toBe('GET');
  });
});
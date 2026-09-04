// @vitest-environment jsdom
/**
 * Jaring pengaman untuk `sanitizeHtml`.
 *
 * Versi sebelumnya adalah no-op total (10/10 payload lolos, 5/5 masukan jinak
 * kembali apa adanya) dan tidak ada satu pun tes yang menahannya. Berkas ini
 * mengunci temuan probe 2026-09-04 sebagai tes yang ikut ter-commit —
 * `sketches/` masuk .gitignore, jadi probe saja tidak bertahan.
 *
 * Butuh DOM: DOMPurify bekerja pada `window`, dan `sanitizeHtml` memang
 * dirancang untuk lingkungan browser (aplikasi ini CSR-only). Environment
 * jsdom di-set lewat docblock di baris pertama karena suite ini secara
 * bawaan berjalan di `node` (`vite.config.js`, blok `test`, tanpa
 * `environment`).
 */
import { describe, it, expect } from 'vitest';
import { sanitizeHtml, stripHtmlToText, ALLOWED_TAGS, ALLOWED_ATTR } from './sanitizeHtml';

/** Payload yang SEMUANYA lolos di implementasi lama. */
const PAYLOAD_LAMA_LOLOS = [
  ['script dibungkus tag tak diizinkan', '<div><script>alert(1)</script></div>'],
  ['script dibungkus dua lapis', '<div><span><script>alert(2)</script></span></div>'],
  ['script telanjang', '<script>alert(3)</script>'],
  ['onerror di dalam pembungkus', '<div><img src=x onerror=alert(4)></div>'],
  ['onerror telanjang', '<img src=x onerror=alert(5)>'],
  ['javascript: href dibungkus', '<div><a href="javascript:alert(6)">klik</a></div>'],
  ['javascript: href telanjang', '<a href="javascript:alert(7)">klik</a>'],
  ['style dibungkus', '<div><style>body{display:none}</style></div>'],
  ['iframe dibungkus', '<div><iframe src="https://jahat.test"></iframe></div>'],
  ['svg onload dibungkus', '<div><svg onload=alert(8)></svg></div>'],
] as const;

describe('sanitizeHtml — payload yang dulu lolos utuh', () => {
  it.each(PAYLOAD_LAMA_LOLOS)('menahan %s', (_nama, masukan) => {
    const keluar = sanitizeHtml(masukan).toLowerCase();
    expect(keluar).not.toContain('<script');
    expect(keluar).not.toContain('<style');
    expect(keluar).not.toContain('<iframe');
    expect(keluar).not.toContain('<svg');
    expect(keluar).not.toContain('javascript:');
    // atribut handler apa pun: onerror, onload, onclick, …
    expect(keluar).not.toMatch(/\son[a-z]+\s*=/);
  });

  it('membuang kode script sepenuhnya, tidak menyisakannya sebagai teks', () => {
    // Bahaya halus: sanitizer yang meng-unwrap <script> akan menampilkan
    // "alert(1)" sebagai teks biasa kepada pembaca.
    expect(sanitizeHtml('<script>alert(1)</script>')).toBe('');
    expect(sanitizeHtml('<div><script>alert(1)</script>halo</div>')).not.toContain('alert(1)');
  });

  it('tidak mengeksekusi handler saat hasilnya disisipkan ke DOM hidup', () => {
    const jejak: string[] = [];
    (globalThis as unknown as { __jejak: string[] }).__jejak = jejak;

    const bersih = sanitizeHtml(
      '<div><img src=x onerror="globalThis.__jejak.push(\'jalan\')"></div>'
    );
    const wadah = document.createElement('div');
    document.body.appendChild(wadah);
    wadah.innerHTML = bersih;

    expect(jejak).toEqual([]);
    wadah.remove();
  });
});

describe('sanitizeHtml — bukan no-op', () => {
  it('meng-unwrap tag yang tidak diizinkan tapi menyimpan teksnya', () => {
    // Jika fungsi ini no-op, ekspektasi berikut gagal — inilah tes yang
    // seharusnya sudah ada sejak awal.
    expect(sanitizeHtml('<div>halo</div>')).toBe('halo');
    expect(sanitizeHtml('<span>teks span</span>')).toBe('teks span');
  });

  it('membuang atribut handler dari tag yang diizinkan', () => {
    const keluar = sanitizeHtml('<p onclick="jahat()">isi</p>');
    expect(keluar).toBe('<p>isi</p>');
  });

  it('membuang href berskema javascript: tapi menyimpan tautannya', () => {
    const keluar = sanitizeHtml('<a href="javascript:void(0)">klik</a>');
    expect(keluar).not.toContain('javascript:');
    expect(keluar).toContain('klik');
  });

  it('membuang atribut style', () => {
    expect(sanitizeHtml('<p style="position:fixed;inset:0">x</p>')).toBe('<p>x</p>');
  });
});

describe('sanitizeHtml — markup sah tetap utuh', () => {
  it('mempertahankan tag pemformatan dasar', () => {
    const masuk = '<p><strong>tebal</strong> dan <em>miring</em></p>';
    expect(sanitizeHtml(masuk)).toBe(masuk);
  });

  it('mempertahankan daftar dan heading', () => {
    const masuk = '<h2>Judul</h2><ul><li>satu</li><li>dua</li></ul>';
    expect(sanitizeHtml(masuk)).toBe(masuk);
  });

  it('mempertahankan tautan http(s) dan mailto', () => {
    for (const href of ['https://contoh.test/a', 'http://contoh.test', 'mailto:a@contoh.test']) {
      const keluar = sanitizeHtml(`<a href="${href}">tautan</a>`);
      expect(keluar).toContain(`href="${href}"`);
    }
  });

  it('memaksa tautan aman: target _blank + rel noopener', () => {
    const keluar = sanitizeHtml('<a href="https://contoh.test">x</a>');
    expect(keluar).toContain('target="_blank"');
    expect(keluar).toContain('rel="nofollow noopener noreferrer"');
  });

  it('tidak menggandakan rel/target saat dipanggil berulang', () => {
    // Hook DOMPurify bersifat global; pemasangan ganda akan menghasilkan
    // atribut yang tertulis dua kali.
    const sekali = sanitizeHtml('<a href="https://contoh.test">x</a>');
    const lagi = sanitizeHtml('<a href="https://contoh.test">x</a>');
    expect(lagi).toBe(sekali);
    expect((lagi.match(/rel=/g) || []).length).toBe(1);
  });

  it('memuat pengumuman produksi tanpa perubahan', () => {
    // Bentuk badan pengumuman nyata di tenant ISP Management (15/16 berformat html).
    expect(sanitizeHtml('<p>test</p>')).toBe('<p>test</p>');
    expect(sanitizeHtml('<p>coba lagi</p>')).toBe('<p>coba lagi</p>');
  });
});

describe('sanitizeHtml — masukan tepi', () => {
  it.each([
    ['string kosong', ''],
    ['hanya spasi', '   '],
  ])('menangani %s', (_nama, masukan) => {
    expect(() => sanitizeHtml(masukan)).not.toThrow();
  });

  it('menangani null/undefined tanpa melempar', () => {
    expect(sanitizeHtml(null as unknown as string)).toBe('');
    expect(sanitizeHtml(undefined as unknown as string)).toBe('');
  });

  it('tidak melempar pada HTML rusak', () => {
    expect(() => sanitizeHtml('<p><strong>belum ditutup')).not.toThrow();
    expect(() => sanitizeHtml('<<>><p')).not.toThrow();
  });
});

describe('stripHtmlToText', () => {
  it('mengembalikan teks polos', () => {
    expect(stripHtmlToText('<p>halo <strong>dunia</strong></p>')).toBe('halo dunia');
  });

  it('tidak menyisakan kode script sebagai teks', () => {
    // Perilaku lama: '<script>alert(1)</script>teks' → 'alert(1)teks'.
    expect(stripHtmlToText('<script>alert(1)</script>teks')).toBe('teks');
    expect(stripHtmlToText('<style>body{color:red}</style>teks')).toBe('teks');
  });

  it('merapikan spasi berlebih', () => {
    expect(stripHtmlToText('<p>a</p>   <p>b</p>')).toBe('a b');
  });

  it('menangani masukan kosong', () => {
    expect(stripHtmlToText('')).toBe('');
    expect(stripHtmlToText(null as unknown as string)).toBe('');
  });
});

describe('kebijakan daftar-izin', () => {
  it('tidak mengizinkan tag pembawa risiko', () => {
    for (const tag of ['script', 'style', 'iframe', 'object', 'embed', 'form', 'input', 'img', 'svg', 'math']) {
      expect(ALLOWED_TAGS).not.toContain(tag);
    }
  });

  it('hanya mengizinkan atribut tautan', () => {
    expect([...ALLOWED_ATTR].sort()).toEqual(['href', 'rel', 'target']);
  });
});

/**
 * Sanitasi HTML tak terpercaya sebelum dirender lewat `{@html …}`.
 *
 * KENAPA FILE INI DITULIS ULANG (2026-09-04)
 *
 * Versi sebelumnya adalah penulis-ulang DOM buatan sendiri, dan terbukti
 * **no-op total** — bukan bocor sebagian, tapi tidak menyaring apa pun:
 *
 *   sanitizeHtml('<script>alert(1)</script>')  →  '<script>alert(1)</script>'
 *   sanitizeHtml('<img src=x onerror=alert(1)>') → utuh, handler tetap jalan
 *   sanitizeHtml('<p onclick="x()">a</p>')     →  utuh
 *
 * 10 dari 10 payload uji lolos, dan 5 dari 5 masukan jinak dikembalikan apa
 * adanya. Akar masalahnya satu baris logika: fungsi itu memasukkan HTML ke
 * `document.createElement('div')` lalu memanggil `walk(root)` pada div
 * container itu sendiri. `div` tidak ada di daftar tag yang diizinkan, jadi
 * cabang "unwrap" langsung dieksekusi pada root; karena root belum menempel ke
 * dokumen, `parentNode` bernilai null sehingga jalur `else` memanggil
 * `el.remove()` lalu `return` — keluar dari `walk()` sebelum satu pun anak
 * dikunjungi. Pohon kembali tanpa disentuh, dan `root.innerHTML` mengembalikan
 * masukan aslinya.
 *
 * Payload yang lolos dirender oleh `AnnouncementDetailView.svelte:109` lewat
 * `{@html sanitizeHtml(announcement.body)}`, di halaman yang dibuka **portal
 * pelanggan** (`(app)/announcements/[id]`) maupun admin. 15 dari 16 pengumuman
 * di data produksi berformat `html`. Token sesi disimpan di `localStorage`
 * (`auth.ts:18` `TOKEN_KEY = 'auth_token'`), jadi skrip yang tereksekusi di
 * halaman itu dapat membacanya.
 *
 * Perbaikannya bukan menambal `walk()`. Menulis sanitizer HTML sendiri berarti
 * bertanding dengan mutation-XSS, namespace SVG/MathML, dan quirk parser —
 * kelas bug yang justru sudah diselesaikan pustaka khusus. Modul ini sekarang
 * mendelegasikan ke DOMPurify (dipin 3.4.14) dan hanya menyimpan kebijakan
 * daftar-izin milik proyek.
 *
 * Catatan yang diverifikasi lewat probe di Chromium, bukan asumsi:
 * `createElement().innerHTML`, `DOMParser`, `createHTMLDocument`, dan
 * `<template>` semuanya **inert** — parsing `<img src=x onerror=…>` tidak
 * mengeksekusi handler sampai node benar-benar disisipkan ke dokumen hidup.
 * Jadi kebocoran lama butuh render `{@html …}`, bukan pemanggilan sanitizer
 * itu sendiri. Duplikasi jejak eksekusi pada probe pertama berasal dari HMR
 * Vite yang memuat modul dua kali, bukan dari sanitizer.
 */
import DOMPurify from 'dompurify';

/**
 * Tag yang boleh bertahan. Sengaja sempit: badan pengumuman ditulis lewat
 * editor teks kaya sederhana, jadi tidak ada alasan mengizinkan `table`,
 * `iframe`, atau `img`. Daftar ini menyalin kebijakan versi lama supaya
 * pengumuman lama tetap tampil sama.
 */
export const ALLOWED_TAGS = [
  'p',
  'br',
  'b',
  'strong',
  'i',
  'em',
  'u',
  's',
  'a',
  'ul',
  'ol',
  'li',
  'blockquote',
  'code',
  'pre',
  'h1',
  'h2',
  'h3',
  'hr',
] as const;

/** Hanya `a` yang boleh beratribut, dan hanya tiga atribut ini. */
export const ALLOWED_ATTR = ['href', 'target', 'rel'] as const;

/**
 * Skema URI yang boleh muncul di `href`. DOMPurify sudah menolak
 * `javascript:` secara bawaan; menyebut daftarnya eksplisit membuat kebijakan
 * proyek terbaca tanpa harus tahu bawaan pustaka.
 */
const ALLOWED_URI_REGEXP = /^(?:(?:https?|mailto):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i;

let hookTerpasang = false;

/**
 * Paksa setiap tautan yang lolos menjadi aman: buka di tab baru tanpa
 * membocorkan `window.opener`, dan tandai `nofollow` karena isinya dikirim
 * pengguna. Dipasang sekali per dokumen; DOMPurify menyimpan hook secara
 * global sehingga pemasangan ganda akan menjalankannya dua kali.
 */
function pasangHook() {
  if (hookTerpasang) return;
  DOMPurify.addHook('afterSanitizeAttributes', (node) => {
    if (node.nodeName === 'A' && node.hasAttribute('href')) {
      node.setAttribute('target', '_blank');
      node.setAttribute('rel', 'nofollow noopener noreferrer');
    }
  });
  hookTerpasang = true;
}

/**
 * Buang seluruh markup dan kembalikan teksnya saja.
 *
 * Dipakai daftar/preview admin dan dashboard. Jalur ini sudah aman sebelumnya
 * (`textContent` tidak pernah mengeksekusi apa pun), tapi cabang non-DOM-nya
 * memakai regex `<[^>]*>` yang meninggalkan isi `<script>` sebagai teks —
 * `'<script>alert(1)</script>teks'` menjadi `'alert(1)teks'`. Membingungkan
 * dalam preview, walau tidak berbahaya. Sekarang isi `script`/`style` dibuang
 * lebih dulu di kedua jalur.
 */
export function stripHtmlToText(input: string): string {
  const teks = String(input ?? '');
  if (!teks) return '';

  if (typeof document === 'undefined') {
    return teks
      .replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi, '')
      .replace(/<[^>]*>/g, '')
      .replace(/\s+/g, ' ')
      .trim();
  }

  // `KEEP_CONTENT: true` + `FORBID_CONTENTS` = teks biasa dipertahankan,
  // tapi isi `script`/`style` ikut terbuang. `KEEP_CONTENT: false` salah di
  // sini: karena ALLOWED_TAGS kosong, SEMUA elemen terlarang sehingga seluruh
  // teks hilang dan fungsi mengembalikan string kosong (terbukti lewat probe).
  const bersih = DOMPurify.sanitize(teks, {
    ALLOWED_TAGS: [],
    ALLOWED_ATTR: [],
    KEEP_CONTENT: true,
    FORBID_CONTENTS: ['script', 'style', 'template', 'noscript'],
  });

  const wadah = document.createElement('div');
  wadah.innerHTML = bersih;
  return (wadah.textContent || '').replace(/\s+/g, ' ').trim();
}

/**
 * Kembalikan HTML yang aman dirender lewat `{@html …}`.
 *
 * Di lingkungan tanpa DOM, mengembalikan HTML mentah sama saja menyerahkan
 * payload ke pemanggil. Aplikasi ini berjalan CSR-only (`+layout.ts:5`
 * `export const ssr = false`), jadi cabang itu hanya tercapai di unit test —
 * dan di sana yang benar adalah gagal aman: buang seluruh tag.
 */
export function sanitizeHtml(input: string): string {
  const teks = String(input ?? '');
  if (!teks) return '';

  if (typeof document === 'undefined') {
    return stripHtmlToText(teks);
  }

  pasangHook();

  return DOMPurify.sanitize(teks, {
    ALLOWED_TAGS: [...ALLOWED_TAGS],
    ALLOWED_ATTR: [...ALLOWED_ATTR],
    ALLOWED_URI_REGEXP,
    // Buang isi elemen terlarang, jangan promosikan jadi teks: `<script>`
    // menjadi hilang seluruhnya, bukan menyisakan kodenya sebagai teks biasa.
    FORBID_CONTENTS: ['script', 'style', 'template', 'noscript'],
    // CATATAN: `USE_PROFILES` sengaja TIDAK dipakai. Terbukti lewat probe
    // (sketches/redesign-2026-09/probe-dompurify.mjs) bahwa menyetel
    // `USE_PROFILES: { html: true }` MENIMPA `ALLOWED_TAGS`/`ALLOWED_ATTR` —
    // hasilnya `<div>`, `<style>`, dan atribut `style` semuanya lolos. Daftar
    // izin eksplisit di atas lebih sempit daripada profil html DOMPurify, jadi
    // profil itu justru melemahkan kebijakan.
    // Jangan biarkan atribut `is`/custom element menyelinap.
    ALLOW_UNKNOWN_PROTOCOLS: false,
    ALLOW_DATA_ATTR: false,
    ALLOW_ARIA_ATTR: false,
  });
}

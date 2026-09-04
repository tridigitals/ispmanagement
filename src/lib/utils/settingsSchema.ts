/**
 * Skema field halaman pengaturan v2.
 *
 * KENAPA INI ADA, bukan sekadar refactor.
 *
 * Halaman lama (`(app)/admin/settings/+page.svelte`, 2.098 baris) menyimpan
 * dua hal di satu tempat: daftar key per kategori DAN markup input per key,
 * ditulis manual dengan `{#if}` panjang. Konsekuensi terukurnya:
 *
 *  1. Nilai default disebar sebagai 27 baris `if (key === '...' && !val) val = ...`
 *     di dalam `buildLocalSettingsFromData()`. Kalau key ditambah tapi baris
 *     default-nya lupa ditulis, field tampil kosong dan tersimpan sebagai
 *     string kosong — bukan nilai wajar.
 *  2. Tipe input tidak pernah dinyatakan. Boolean muncul sebagai `<select>`
 *     "true"/"false", angka sebagai `type="text"`, rahasia sebagai teks biasa.
 *     Password SMTP dan secret key pembayaran terbaca di layar.
 *  3. `saveChanges()` mengirim `categories[activeTab].keys` saja, sedangkan
 *     `hasChanges` dihitung dari semua kategori. Terkonfirmasi lewat probe di
 *     viewport mobile: edit di General membuat Simpan menyala, lalu pindah ke
 *     Network dan menekan Simpan menyimpan key Network dan menimpa edit
 *     General. Di desktop jalurnya membuang edit (`discard: true`), di mobile
 *     mempertahankannya — dua perilaku berlawanan di halaman yang sama.
 *
 * Skema di bawah menjadikan ketiganya data, bukan markup: tipe, default,
 * satuan, dan teks bantuan hidup di satu tempat dan bisa diuji. Halaman v2
 * merender dari skema ini, jadi menambah pengaturan tidak lagi berarti
 * menambah cabang `{#if}`.
 */

import type { FieldOption, FieldType, IconName } from '$lib/components/ds';

export interface SettingField {
  key: string;
  label: string;
  type: FieldType;
  /** Dipakai kalau nilai dari server kosong. Menggantikan 27 baris if-default. */
  fallback?: string;
  help?: string;
  options?: FieldOption[];
  placeholder?: string;
  suffix?: string;
  min?: number;
  max?: number;
  rows?: number;
  /** Bergantung pada field lain: hanya tampil kalau syaratnya benar. */
  visibleWhen?: { key: string; equals: string };
}

export interface SettingSection {
  id: string;
  label: string;
  icon: IconName;
  desc: string;
  fields: SettingField[];
}

const BOOL = (fallback = 'false'): Pick<SettingField, 'type' | 'fallback'> => ({
  type: 'toggle',
  fallback,
});

/**
 * Bagian yang dirender langsung dari skema.
 *
 * Sengaja TIDAK memuat branding/domain, billing & plan, dan event notifications:
 * ketiganya bukan daftar key sederhana melainkan panel dengan alur sendiri
 * (upload logo, status domain kustom, matriks event × kanal). Memaksakan
 * ketiganya ke skema hanya akan memindahkan kerumitan, bukan menghapusnya —
 * itu tetap komponen terpisah.
 */
export const SETTING_SECTIONS: SettingSection[] = [
  {
    id: 'general',
    label: 'Umum',
    icon: 'cog',
    desc: 'Identitas aplikasi, bahasa, dan mata uang yang dipakai di seluruh tenant.',
    fields: [
      { key: 'app_name', label: 'Nama aplikasi', type: 'text', placeholder: 'ISP Management' },
      {
        key: 'app_description',
        label: 'Deskripsi',
        type: 'textarea',
        rows: 2,
        help: 'Muncul di halaman login dan judul tab browser.',
      },
      {
        key: 'support_email',
        label: 'Email dukungan',
        type: 'email',
        placeholder: 'support@contoh.com',
        help: 'Alamat yang ditampilkan ke pelanggan saat butuh bantuan.',
      },
      {
        key: 'default_locale',
        label: 'Bahasa bawaan',
        type: 'select',
        fallback: 'id',
        options: [
          { value: 'id', label: 'Bahasa Indonesia' },
          { value: 'en', label: 'English (US)' },
        ],
      },
      {
        key: 'currency_code',
        label: 'Mata uang',
        type: 'select',
        fallback: 'IDR',
        options: [
          { value: 'IDR', label: 'IDR — Rupiah' },
          { value: 'USD', label: 'USD — Dolar AS' },
        ],
        help: 'Mengubah format angka di tagihan dan laporan.',
      },
    ],
  },
  {
    id: 'company',
    label: 'Perusahaan',
    icon: 'clipboard',
    desc: 'Data yang tercetak di tagihan dan kuitansi.',
    fields: [
      { key: 'organization_name', label: 'Nama badan usaha', type: 'text' },
      { key: 'company_address', label: 'Alamat', type: 'textarea', rows: 3 },
      { key: 'company_phone', label: 'Telepon', type: 'text' },
      { key: 'company_email', label: 'Email', type: 'email' },
      { key: 'company_npwp', label: 'NPWP', type: 'text' },
      { key: 'company_website', label: 'Situs web', type: 'text', placeholder: 'https://' },
      {
        key: 'invoice_footer_note',
        label: 'Catatan kaki tagihan',
        type: 'textarea',
        rows: 2,
        help: 'Tampil di bagian bawah setiap tagihan yang dicetak.',
      },
    ],
  },
  {
    id: 'security',
    label: 'Keamanan',
    icon: 'shield',
    desc: 'Verifikasi email dan pendaftaran mandiri pelanggan.',
    fields: [
      {
        key: 'auth_require_email_verification',
        label: 'Wajib verifikasi email',
        ...BOOL(),
        help: 'Butuh konfigurasi Email yang sudah berjalan. Tanpa itu pengguna baru tidak bisa masuk.',
      },
      {
        key: 'customer_self_registration_enabled',
        label: 'Pendaftaran mandiri pelanggan',
        ...BOOL(),
        help: 'Mengizinkan calon pelanggan membuat akun sendiri lewat portal.',
      },
    ],
  },
  {
    id: 'network',
    label: 'Jaringan',
    icon: 'activity',
    desc: 'Ambang peringatan router dan aturan eskalasi insiden.',
    fields: [
      { key: 'mikrotik_alerting_enabled', label: 'Peringatan router aktif', ...BOOL('true') },
      {
        key: 'mikrotik_alert_offline_after_secs',
        label: 'Dianggap offline setelah',
        type: 'number',
        fallback: '60',
        suffix: 'detik',
        min: 10,
        visibleWhen: { key: 'mikrotik_alerting_enabled', equals: 'true' },
      },
      {
        key: 'mikrotik_alert_cpu_risk',
        label: 'CPU waspada',
        type: 'number',
        fallback: '70',
        suffix: '%',
        min: 1,
        max: 100,
        visibleWhen: { key: 'mikrotik_alerting_enabled', equals: 'true' },
      },
      {
        key: 'mikrotik_alert_cpu_hot',
        label: 'CPU kritis',
        type: 'number',
        fallback: '85',
        suffix: '%',
        min: 1,
        max: 100,
        visibleWhen: { key: 'mikrotik_alerting_enabled', equals: 'true' },
      },
      {
        key: 'mikrotik_alert_latency_risk_ms',
        label: 'Latensi waspada',
        type: 'number',
        fallback: '200',
        suffix: 'ms',
        min: 1,
        visibleWhen: { key: 'mikrotik_alerting_enabled', equals: 'true' },
      },
      {
        key: 'mikrotik_alert_latency_hot_ms',
        label: 'Latensi kritis',
        type: 'number',
        fallback: '400',
        suffix: 'ms',
        min: 1,
        visibleWhen: { key: 'mikrotik_alerting_enabled', equals: 'true' },
      },
      {
        key: 'mikrotik_incident_sla_warn_minutes',
        label: 'SLA peringatan',
        type: 'number',
        fallback: '30',
        suffix: 'menit',
        min: 1,
        help: 'Insiden yang belum ditangani selama ini akan ditandai.',
      },
      {
        key: 'mikrotik_incident_sla_breach_minutes',
        label: 'SLA terlampaui',
        type: 'number',
        fallback: '120',
        suffix: 'menit',
        min: 1,
        help: 'Harus lebih besar dari SLA peringatan.',
      },
      {
        key: 'mikrotik_incident_correlation_enabled',
        label: 'Korelasi insiden',
        ...BOOL('true'),
        help: 'Menggabungkan beberapa peringatan dari router yang sama menjadi satu insiden.',
      },
      {
        key: 'mikrotik_incident_auto_escalation_enabled',
        label: 'Eskalasi otomatis',
        ...BOOL(),
      },
      {
        key: 'mikrotik_incident_escalation_minutes',
        label: 'Eskalasi setelah',
        type: 'number',
        fallback: '60',
        suffix: 'menit',
        min: 1,
        visibleWhen: { key: 'mikrotik_incident_auto_escalation_enabled', equals: 'true' },
      },
      { key: 'mikrotik_alert_email_enabled', label: 'Kirim peringatan via email', ...BOOL() },
      {
        key: 'mikrotik_incident_assignment_email_enabled',
        label: 'Email saat insiden ditugaskan',
        ...BOOL(),
      },
      {
        key: 'pppoe_auto_apply_on_save_enabled',
        label: 'Terapkan PPPoE otomatis saat disimpan',
        ...BOOL(),
        help: 'Perubahan akun langsung dikirim ke router tanpa langkah Terapkan manual.',
      },
    ],
  },
  {
    id: 'storage',
    label: 'Penyimpanan',
    icon: 'database',
    desc: 'Lokasi penyimpanan berkas unggahan.',
    fields: [
      {
        key: 'storage_driver',
        label: 'Driver',
        type: 'select',
        fallback: 'system',
        options: [
          { value: 'system', label: 'Bawaan sistem (dikelola)' },
          { value: 's3', label: 'AWS S3' },
          { value: 'r2', label: 'Cloudflare R2' },
        ],
      },
      {
        key: 'storage_s3_bucket',
        label: 'Bucket',
        type: 'text',
        visibleWhen: { key: 'storage_driver', equals: 's3' },
      },
      {
        key: 'storage_s3_region',
        label: 'Region',
        type: 'text',
        visibleWhen: { key: 'storage_driver', equals: 's3' },
      },
      {
        key: 'storage_s3_endpoint',
        label: 'Endpoint',
        type: 'text',
        placeholder: 'https://',
        visibleWhen: { key: 'storage_driver', equals: 's3' },
      },
      {
        key: 'storage_s3_access_key',
        label: 'Access key',
        type: 'text',
        visibleWhen: { key: 'storage_driver', equals: 's3' },
      },
      {
        key: 'storage_s3_secret_key',
        label: 'Secret key',
        /* Halaman lama merender ini sebagai teks biasa sehingga terbaca di layar. */
        type: 'password',
        visibleWhen: { key: 'storage_driver', equals: 's3' },
      },
      {
        key: 'storage_s3_public_url',
        label: 'URL publik',
        type: 'text',
        placeholder: 'https://',
        visibleWhen: { key: 'storage_driver', equals: 's3' },
      },
    ],
  },
];

/** Bagian yang tetap memakai komponen sendiri, bukan skema. */
export const PANEL_SECTIONS: ReadonlyArray<{
  id: string;
  label: string;
  icon: IconName;
  desc: string;
}> = [
  { id: 'branding', label: 'Merek & Domain', icon: 'monitor', desc: 'Logo, nama tampilan, dan domain kustom.' },
  { id: 'billing_plan', label: 'Tagihan & Paket', icon: 'card', desc: 'Paket langganan tenant ini.' },
  { id: 'email', label: 'Email', icon: 'mail', desc: 'Pengirim email keluar dan uji koneksi.' },
  { id: 'payment', label: 'Pembayaran', icon: 'card', desc: 'Gerbang pembayaran dan rekening manual.' },
  { id: 'service', label: 'Layanan', icon: 'wifi', desc: 'Penerbitan tagihan otomatis dan penangguhan.' },
  { id: 'whatsapp', label: 'WhatsApp', icon: 'megaphone', desc: 'Gateway WhatsApp untuk notifikasi.' },
  {
    id: 'event_notifications',
    label: 'Notifikasi Event',
    icon: 'bell',
    desc: 'Event mana yang dikirim ke kanal mana.',
  },
];

/** Semua key yang dikelola skema — dipakai untuk menghitung perubahan. */
export function schemaKeys(): string[] {
  return SETTING_SECTIONS.flatMap((s) => s.fields.map((f) => f.key));
}

/**
 * Nilai awal sebuah field: nilai server kalau ada, kalau tidak `fallback`.
 *
 * Ini menggantikan 27 baris `if (key === '...' && !val) val = '...'` di halaman
 * lama. Default hidup bersama definisi field-nya, jadi tidak bisa lupa.
 */
export function initialValue(field: SettingField, serverValue: string | undefined): string {
  const raw = (serverValue ?? '').trim();
  if (raw !== '') return raw;
  return field.fallback ?? '';
}

/** Field terlihat atau tidak, berdasarkan nilai field lain. */
export function isVisible(field: SettingField, values: Record<string, string>): boolean {
  if (!field.visibleWhen) return true;
  return values[field.visibleWhen.key] === field.visibleWhen.equals;
}

/**
 * Validasi lintas field yang halaman lama tidak punya sama sekali.
 *
 * Terukur di DB: `mikrotik_incident_sla_breach_minutes` bisa disimpan lebih
 * kecil dari `..._sla_warn_minutes`, dan halaman lama hanya menutupinya di
 * tampilan pratinjau (`slaBreachPreview` mengalikan warn × 2 kalau breach lebih
 * kecil) — jadi yang tersimpan tetap nilai tidak masuk akal.
 */
export function validate(values: Record<string, string>): Record<string, string> {
  const errors: Record<string, string> = {};

  const warn = Number.parseInt(values['mikrotik_incident_sla_warn_minutes'] ?? '', 10);
  const breach = Number.parseInt(values['mikrotik_incident_sla_breach_minutes'] ?? '', 10);
  if (Number.isFinite(warn) && Number.isFinite(breach) && breach <= warn) {
    errors['mikrotik_incident_sla_breach_minutes'] =
      `Harus lebih besar dari SLA peringatan (${warn} menit).`;
  }

  const cpuRisk = Number.parseInt(values['mikrotik_alert_cpu_risk'] ?? '', 10);
  const cpuHot = Number.parseInt(values['mikrotik_alert_cpu_hot'] ?? '', 10);
  if (Number.isFinite(cpuRisk) && Number.isFinite(cpuHot) && cpuHot <= cpuRisk) {
    errors['mikrotik_alert_cpu_hot'] = `Harus lebih besar dari CPU waspada (${cpuRisk}%).`;
  }

  const latRisk = Number.parseInt(values['mikrotik_alert_latency_risk_ms'] ?? '', 10);
  const latHot = Number.parseInt(values['mikrotik_alert_latency_hot_ms'] ?? '', 10);
  if (Number.isFinite(latRisk) && Number.isFinite(latHot) && latHot <= latRisk) {
    errors['mikrotik_alert_latency_hot_ms'] = `Harus lebih besar dari latensi waspada (${latRisk} ms).`;
  }

  if (values['storage_driver'] === 's3') {
    for (const [key, label] of [
      ['storage_s3_bucket', 'Bucket'],
      ['storage_s3_access_key', 'Access key'],
      ['storage_s3_secret_key', 'Secret key'],
    ] as const) {
      if (!(values[key] ?? '').trim()) errors[key] = `${label} wajib diisi untuk driver S3.`;
    }
  }

  return errors;
}

/**
 * Store settings TUNGGAL yang dipakai halaman v2 DAN halaman lama.
 *
 * KENAPA: versi lama menyimpan state-nya sendiri (`localSettings` 2.098 baris
 * di +page.svelte). Ketika halaman v2 me-mount panel lama sebagai komponen
 * (email, payment, service, whatsapp, event_notifications, billing plan),
 * keduanya harus berbagi SATU sumber kebenaran — dua `localSettings` berarti
 * dua baseline dan simpan yang saling menimpa.
 *
 * Kesepakatan yang diganti di sini juga konkret: halaman lama `saveChanges()`
 * hanya mengirim key tab aktif (`categories[activeTab].keys`) padahal
 * `hasChanges` dihitung dari semua tab — edit lintas tab hilang diam-diam
 * (lihat komentar di (v2)/v2/admin/settings/+page.svelte). Store ini mengirim
 * SEMUA key yang berubah terhadap baseline, plus patch tenant & logo.
 *
 * Persistensi `adminSettingsCache` (store localStorage legacy) sengaja TIDAK
 * ditiru: ia hanya cache bootstrap dan halaman lama masih menulisnya sendiri.
 */
import { api } from '$lib/api/client';
import { appLogo } from '$lib/stores/logo';
import { getToken } from '$lib/stores/auth';
import type { EmailVerificationReadiness, Setting } from '$lib/api/client';
import { WHATSAPP_GATEWAY_SETTING_KEYS } from '$lib/utils/whatsappGateway';
import {
  SETTING_SECTIONS,
  initialValue,
  schemaKeys,
} from '$lib/utils/settingsSchema';

export interface TenantPatch {
  name?: string;
  customDomain?: string;
  enforce2fa?: boolean;
}

/** Key panel lama yang bukan bagian dari schema v2, per seksi. */
export const PANEL_SETTING_KEYS: Record<string, string[]> = {
  company: ['company_logo', 'company_whatsapp'],
  email: [
    'email_provider',
    'email_smtp_host',
    'email_smtp_port',
    'email_smtp_username',
    'email_smtp_password',
    'email_smtp_encryption',
    'email_api_key',
    'email_from_address',
    'email_from_name',
    'email_webhook_url',
  ],
  payment: [
    'payment_midtrans_enabled',
    'payment_midtrans_merchant_id',
    'payment_midtrans_client_key',
    'payment_midtrans_server_key',
    'payment_midtrans_is_production',
    'payment_duitku_enabled',
    'payment_duitku_merchant_code',
    'payment_duitku_api_key',
    'payment_duitku_payment_method',
    'payment_duitku_payment_methods',
    'payment_duitku_is_production',
    'payment_manual_enabled',
    'payment_manual_instructions',
    'payment_manual_accounts',
  ],
  service: [
    'customer_invoice_auto_generate_enabled',
    'customer_invoice_generate_days_before_due',
    'customer_invoice_scheduler_interval_minutes',
    'customer_invoice_last_run_at',
    'billing_auto_suspend_enabled',
    'billing_auto_suspend_mode',
    'billing_auto_suspend_grace_days',
    'billing_auto_suspend_fixed_day',
    'billing_auto_suspend_pppoe_action',
    'billing_auto_resume_on_payment',
    'billing_reminder_enabled',
    'billing_reminder_schedule',
  ],
  whatsapp: [...WHATSAPP_GATEWAY_SETTING_KEYS],
  event_notifications: ['wa_events_tenant'],
};

/** Default skema (fallback field) ikut ke peta yang sama utk guard changedKeys. */
const SCHEMA_FALLBACKS: Record<string, string> = Object.fromEntries(
  SETTING_SECTIONS.flatMap((sec) => sec.fields)
    .filter((f) => f.fallback)
    .map((f) => [f.key, f.fallback!]),
);

const FALLBACKS: Record<string, string> = {
  email_provider: 'smtp',
  email_smtp_port: '587',
  email_smtp_encryption: 'starttls',
  payment_midtrans_is_production: 'true',
  payment_duitku_is_production: 'true',
  payment_duitku_payment_method: 'qris',
  customer_invoice_auto_generate_enabled: 'true',
  customer_invoice_generate_days_before_due: '7',
  customer_invoice_scheduler_interval_minutes: '60',
  billing_auto_suspend_mode: 'grace_period',
  billing_auto_suspend_grace_days: '3',
  billing_auto_suspend_fixed_day: '1',
  billing_auto_suspend_pppoe_action: 'disable',
  billing_auto_resume_on_payment: 'true',
  billing_reminder_enabled: 'true',
  billing_reminder_schedule: 'H-3,H-1,H+1,H+3',
  wa_gateway_enabled: 'false',
  wa_gateway_provider: 'disabled',
  wa_events_tenant: '{}',
};

export function allSettingKeys(): string[] {
  return [...schemaKeys(), ...Object.values(PANEL_SETTING_KEYS).flat()];
}

/* Satu instance per proses. Halaman v2 dan legacy berbagi objek yang sama:
   dua `localSettings` berarti dua baseline dan simpan yang saling menimpa.
   File .svelte.ts => runes $state legal di module scope (Svelte 5). */
export const settingsStore = $state({
  loading: true,
  saving: false,
  values: {} as Record<string, string>,
  /** Nilai APA ADANYA di server (bukan hasil fallback) — dasar changedKeys. */
  baseline: {} as Record<string, string>,
  /** Row server mentah, untuk tahu key mana yang punya baris Setting. */
  rows: {} as Record<string, Setting>,
  tenantInfo: null as any,
  customDomainAccess: false,
  emailReadiness: { ready: true, reason: null } as EmailVerificationReadiness,
  logoBase64: null as string | null,
  tenantPatch: {} as TenantPatch,
  logoDirty: false,
  loadError: null as string | null,
});

function applyLoaded(data: Setting[], tenant: any, access: boolean, logo: string | null) {
  settingsStore.rows = data.reduce(
    (acc, curr) => {
      acc[curr.key] = curr;
      return acc;
    },
    {} as Record<string, Setting>,
  );
  settingsStore.tenantInfo = tenant;
  settingsStore.customDomainAccess = access;
  settingsStore.logoBase64 = logo;
  settingsStore.logoDirty = false;
  settingsStore.tenantPatch = {};

  const byKey = new Map(data.map((r) => [r.key, r.value ?? '']));
  const next: Record<string, string> = {};
  const base: Record<string, string> = {};

  for (const s of SETTING_SECTIONS) {
    for (const f of s.fields) {
      next[f.key] = initialValue(f, byKey.get(f.key));
      base[f.key] = (byKey.get(f.key) ?? '').trim();
    }
  }
  for (const key of Object.values(PANEL_SETTING_KEYS).flat()) {
    next[key] = (byKey.get(key) ?? '').trim() || FALLBACKS[key] || '';
    base[key] = (byKey.get(key) ?? '').trim();
  }
  // tenant-owned values live in values[] too so panels can bind uniformly
  next['tenant_name'] = tenant?.name || '';
  next['custom_domain'] = tenant?.custom_domain || '';
  next['enforce_2fa'] = String(tenant?.enforce_2fa ?? false);

  settingsStore.values = next;
  settingsStore.baseline = base;
}


export async function loadSettings(): Promise<void> {
  settingsStore.loading = true;
  settingsStore.loadError = null;
  try {
    const token = getToken() || undefined;
    let logoStoreValue: string | null = null;
    appLogo.subscribe((v) => (logoStoreValue = v))();

    const [, data, tenant, readiness] = await Promise.all([
      appLogo.refresh(token).catch(() => null),
      api.settings.getAll(),
      api.tenant.getSelf(),
      api.settings
        .getEmailVerificationReadiness()
        .catch(() => ({ ready: true, reason: null }) as EmailVerificationReadiness),
    ]);
    const access = await api.plans
      .checkAccess(tenant.id, 'custom_domain')
      .catch(() => ({ has_access: false }) as any);
    let logoAfter: string | null = null;
    appLogo.subscribe((v) => (logoAfter = v))();

    applyLoaded(data, tenant, Boolean(access?.has_access), logoAfter || logoStoreValue || null);
    settingsStore.emailReadiness = readiness;
  } catch (error: any) {
    settingsStore.loadError = error?.message || String(error);
    throw error;
  } finally {
    settingsStore.loading = false;
  }
}

export function changedKeys(): string[] {
  const keys: string[] = [];
  for (const k of allSettingKeys()) {
    const value = settingsStore.values[k] ?? '';
    const base = settingsStore.baseline[k] ?? '';
    if (value === base) continue;
    /* Panel lama menampilkan fallback saat server kosong (mis.
       wa_gateway_provider='disabled', payment_duitku_payment_method='qris').
       Itu bukan suntingan user — jangan nyalakan SaveBar selamanya untuk
       key yang memang belum pernah disimpan. */
    if (base === '' && (FALLBACKS[k] ?? SCHEMA_FALLBACKS[k]) === value) continue;
    keys.push(k);
  }
  if (Object.keys(settingsStore.tenantPatch).length > 0) keys.push('tenant');
  if (settingsStore.logoDirty) keys.push('app_logo_path');
  return keys;
}

export function hasAnyChanges(): boolean {
  return changedKeys().length > 0;
}

/** Guard warisan: verifikasi email tidak bisa aktif tanpa SMTP siap. */
export function changeSetting(key: string, value: unknown): boolean {
  if (
    key === 'auth_require_email_verification' &&
    Boolean(value) &&
    !settingsStore.emailReadiness.ready
  ) {
    return false; // caller menampilkan toast alasan
  }
  settingsStore.values[key] = String(value);

  const tenant = settingsStore.tenantInfo;
  if (key === 'tenant_name') {
    settingsStore.tenantPatch.name = String(value);
    if (value === (tenant?.name || '')) delete settingsStore.tenantPatch.name;
  }
  if (key === 'custom_domain') {
    settingsStore.tenantPatch.customDomain = String(value);
    if (value === (tenant?.custom_domain || '')) delete settingsStore.tenantPatch.customDomain;
  }
  if (key === 'enforce_2fa') {
    settingsStore.tenantPatch.enforce2fa = Boolean(value);
    if (Boolean(value) === Boolean(tenant?.enforce_2fa ?? false))
      delete settingsStore.tenantPatch.enforce2fa;
  }
  return true;
}

export interface SaveResult {
  ok: boolean;
  savedCount: number;
  failedCount: number;
  totalCount: number;
}

/**
 * Simpan SEMUA perubahan lintas tab (bukan hanya tab aktif) — perbedaan inti
 * dari `saveChanges()` legacy.
 */
export async function saveAllSettings(): Promise<SaveResult> {
  const changed = changedKeys();
  if (changed.length === 0) return { ok: true, savedCount: 0, failedCount: 0, totalCount: 0 };

  settingsStore.saving = true;
  try {
    if (Object.keys(settingsStore.tenantPatch).length > 0) {
      await api.tenant.updateSelf(settingsStore.tenantPatch);
    }

    const settingKeys = changed.filter((k) => k !== 'tenant' && k !== 'app_logo_path');
    let failed = 0;
    const results = await Promise.allSettled(
      settingKeys.map((k) => api.settings.upsert(k, settingsStore.values[k] ?? '')),
    );
    failed = results.filter((r) => r.status === 'rejected').length;

    /* Baca ulang penuh supaya baseline baru = keadaan server sesungguhnya;
       key yang gagal tetap terlihat berubah dan SaveBar tetap menyala utk itu. */
    await loadSettings();
    return {
      ok: failed === 0,
      savedCount: settingKeys.length - failed,
      failedCount: failed,
      totalCount: settingKeys.length,
    };
  } finally {
    settingsStore.saving = false;
  }
}

export function discardSettings(): void {
  applyReloadBaseline();
}

/** Restore values[] dari baseline + tenantInfo tanpa network. */
function applyReloadBaseline() {
  const next = { ...settingsStore.values };
  for (const k of allSettingKeys()) next[k] = settingsStore.baseline[k] ?? next[k] ?? '';
  next['tenant_name'] = settingsStore.tenantInfo?.name || '';
  next['custom_domain'] = settingsStore.tenantInfo?.custom_domain || '';
  next['enforce_2fa'] = String(settingsStore.tenantInfo?.enforce_2fa ?? false);
  settingsStore.values = next;
  settingsStore.tenantPatch = {};
  settingsStore.logoDirty = false;
}

export async function uploadLogo(dataUrlBase64: string): Promise<string> {
  const base64Data = dataUrlBase64.split(',')[1];
  const path = await api.settings.uploadLogo(base64Data);
  settingsStore.values['app_logo_path'] = path;
  settingsStore.logoBase64 = dataUrlBase64;
  settingsStore.logoDirty = true;
  appLogo.set(dataUrlBase64);
  return path;
}

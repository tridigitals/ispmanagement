import { toast as sonnerToast } from 'svelte-sonner';

// ── A-08: chokepoint pesan error user-facing ───────────────────────────────
// Semua `toast.error(...)` app-wide melewati sini. String yang terlihat
// internals backend (sqlx, stack, path, DB detail) ditukar pesan umum —
// developer tetap bisa baca detail di console/log (extractApiErrorMessage),
// tapi tidak di layar user. toast.warning juga ikut disanitasi (dipakai
// untuk PLAN_FEATURE_REQUIRED dsb yang kadang membawa raw upstream).
const INTERNAL_RE =
  /\b(database error|internal server error|internal error|connection refused|connection reset|connection closed|fatal error|panic[.:\s]|stack trace|traceback|sqlx|tokio|postgres|relation .* does not exist|syntax error at|column .* does not exist|table .* not found|constraint|violat\w+|deadlock|EOF occurred|unexpected end|too many connections|could not (connect|resolve|bind|serialize)|failed to (bind|serialize|query|decode)|deadline exceeded|invalid (input syntax|text representation)|bytea|jsonb|\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:\d+\b)/i;

const SAFE_FALLBACK = 'Terjadi kesalahan pada server. Coba lagi.';

function userSafe(message: unknown): string {
  const text = typeof message === 'string' ? message : String(message ?? '');
  if (!text.trim()) return SAFE_FALLBACK;
  // Sertakan potongan pertama untuk debugging via devtools TANPA membocorkan
  // internals lengkap: cukup kelas kesalahannya.
  return INTERNAL_RE.test(text) ? SAFE_FALLBACK : text;
}

type ToastOptions = NonNullable<Parameters<typeof sonnerToast>[1]>;

function sanitizeOptions<T extends ToastOptions | undefined>(opts: T): T {
  if (opts && typeof (opts as any).description === 'string') {
    return { ...opts, description: userSafe((opts as any).description) } as T;
  }
  return opts;
}

export const toast = Object.assign(
  (...args: Parameters<typeof sonnerToast>) => sonnerToast(...args),
  {
    ...sonnerToast,
    error: (message: any, options?: ToastOptions) =>
      sonnerToast.error(userSafe(message), sanitizeOptions(options)),
    warning: (message: any, options?: ToastOptions) =>
      sonnerToast.warning(userSafe(message), sanitizeOptions(options)),
    // sukses/info/loading lewat tanpa perubahan
    success: sonnerToast.success.bind(sonnerToast),
    info: sonnerToast.info.bind(sonnerToast),
    loading: sonnerToast.loading.bind(sonnerToast),
    dismiss: sonnerToast.dismiss.bind(sonnerToast),
  },
);

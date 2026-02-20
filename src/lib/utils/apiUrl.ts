function normalizeApiBase(url?: string): string | null {
  if (!url) return null;
  const trimmed = String(url).trim();
  if (!trimmed) return null;
  return trimmed.replace(/\/+$/, '');
}

function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  const w = window as any;
  if (w.__TAURI_INTERNALS__ || w.__TAURI__) return true;
  if (typeof navigator !== 'undefined' && typeof navigator.userAgent === 'string') {
    return navigator.userAgent.toLowerCase().includes('tauri');
  }
  return false;
}

export function getApiBaseUrl(): string {
  const configured = normalizeApiBase(import.meta.env.VITE_API_URL);
  if (configured) return configured;

  if (typeof window !== 'undefined' && !isTauriRuntime()) {
    const proto = window.location.protocol;
    if (proto === 'http:' || proto === 'https:') {
      return `${window.location.origin.replace(/\/+$/, '')}/api`;
    }
  }

  return 'http://localhost:3000/api';
}

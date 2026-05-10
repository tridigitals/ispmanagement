import { afterEach, describe, expect, it, vi } from 'vitest';

describe('getApiBaseUrl', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  it('uses current browser origin for web runtime even when VITE_API_URL is set', async () => {
    vi.stubEnv('VITE_API_URL', 'https://api.example.com/api');
    vi.stubGlobal('window', {
      location: {
        origin: 'https://portal.acme.net',
        protocol: 'https:',
      },
    });
    vi.stubGlobal('navigator', { userAgent: 'Mozilla/5.0' });

    const { getApiBaseUrl } = await import('./apiUrl');

    expect(getApiBaseUrl()).toBe('https://portal.acme.net/api');
  });

  it('uses explicit API URL in tauri runtime when configured', async () => {
    vi.stubEnv('VITE_API_URL', 'https://api.example.com/api');
    vi.stubGlobal('window', {
      location: {
        origin: 'https://portal.acme.net',
        protocol: 'https:',
      },
      __TAURI_INTERNALS__: {},
    });
    vi.stubGlobal('navigator', { userAgent: 'Mozilla/5.0' });

    const { getApiBaseUrl } = await import('./apiUrl');

    expect(getApiBaseUrl()).toBe('https://api.example.com/api');
  });

  it('falls back to localhost API when window is unavailable', async () => {
    vi.stubEnv('VITE_API_URL', '');

    const { getApiBaseUrl } = await import('./apiUrl');

    expect(getApiBaseUrl()).toBe('http://localhost:3000/api');
  });
});

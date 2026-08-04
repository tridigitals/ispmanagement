import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const ORIGINAL_ENV = { ...process.env };

afterEach(() => {
  vi.restoreAllMocks();
  vi.resetModules();
  process.env = { ...ORIGINAL_ENV };
});

beforeEach(() => {
  // Keep repository `.env` tunnel defaults out of unit-test configuration.
  process.env.VITE_ALLOWED_HOSTS = '';
  process.env.ALLOW_UNSAFE_PUBLIC_DEV = '';
});

describe('vite dev proxy', () => {
  it('enables websocket upgrades for the /api proxy when VITE_API_URL is set', async () => {
    process.env.VITE_API_URL = 'http://localhost:3000/api';

    const { default: createConfig } = await import('./vite.config.js');
    const config = await createConfig({ mode: 'test' });

    expect(config.server?.proxy?.['/api']).toMatchObject({
      target: 'http://localhost:3000',
      changeOrigin: true,
      secure: false,
      ws: true,
    });
  });

  it('warns when vite dev is exposed through non-local allowed hosts', async () => {
    process.env.VITE_ALLOWED_HOSTS = 'billing.tridigitals.com';
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const { default: createConfig } = await import('./vite.config.js');
    await createConfig({ mode: 'test' });

    expect(warn).toHaveBeenCalledWith(
      expect.stringContaining('Vite dev server is configured for non-local browser access'),
    );
  });

  it('refuses wildcard allowed hosts without an explicit unsafe override', async () => {
    process.env.VITE_ALLOWED_HOSTS = 'all';

    const { default: createConfig } = await import('./vite.config.js');

    await expect(createConfig({ mode: 'test' })).rejects.toThrow(
      /Refusing to start Vite dev server with VITE_ALLOWED_HOSTS=all/,
    );
  });

  it('allows wildcard allowed hosts when the unsafe override is set', async () => {
    process.env.VITE_ALLOWED_HOSTS = 'all';
    process.env.ALLOW_UNSAFE_PUBLIC_DEV = '1';
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const { default: createConfig } = await import('./vite.config.js');
    const config = await createConfig({ mode: 'test' });

    expect(config.server?.allowedHosts).toBe(true);
    expect(warn).toHaveBeenCalledWith(
      expect.stringContaining('ALLOW_UNSAFE_PUBLIC_DEV=1'),
    );
  });
});

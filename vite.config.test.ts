import { describe, expect, it } from 'vitest';

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
});

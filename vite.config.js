import { defineConfig, loadEnv } from 'vite';
// @ts-nocheck
import { sveltekit } from '@sveltejs/kit/vite';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const host = process.env.TAURI_DEV_HOST;
const LOCAL_DEV_HOSTS = new Set(['localhost', '127.0.0.1', '::1']);

/**
 * @param {unknown} rawValue
 * @returns {string[]}
 */
function csvList(rawValue) {
  return String(rawValue || '')
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean);
}

/**
 * @param {unknown} value
 * @returns {boolean}
 */
function isLocalHostname(value) {
  return LOCAL_DEV_HOSTS.has(String(value || '').trim().toLowerCase());
}

/**
 * @param {string[]} explicitAllowedHosts
 * @param {string[]} corsOrigins
 * @param {string | undefined} devHost
 * @returns {string[]}
 */
function collectPublicDevHosts(explicitAllowedHosts, corsOrigins, devHost) {
  const publicHosts = new Set();

  for (const allowedHost of explicitAllowedHosts) {
    const normalized = allowedHost.toLowerCase();
    if (normalized !== 'all' && !isLocalHostname(normalized)) {
      publicHosts.add(allowedHost);
    }
  }

  for (const origin of corsOrigins) {
    try {
      const hostname = new URL(origin.trim()).hostname;
      if (!isLocalHostname(hostname)) {
        publicHosts.add(hostname);
      }
    } catch {
      // Ignore invalid origins in env parsing. Vite will keep working with the valid ones.
    }
  }

  if (devHost && !isLocalHostname(devHost)) {
    publicHosts.add(devHost);
  }

  return [...publicHosts];
}

/**
 * @param {string[]} publicDevHosts
 * @param {boolean} wildcardHostsEnabled
 * @returns {void}
 */
function warnPublicDevExposure(publicDevHosts, wildcardHostsEnabled) {
  if (wildcardHostsEnabled) {
    console.warn(
      '[vite] Vite dev server is configured with VITE_ALLOWED_HOSTS=all. This is unsafe outside intentional tunnel/debug usage. Set ALLOW_UNSAFE_PUBLIC_DEV=1 only when you explicitly accept that risk.',
    );
    return;
  }

  if (publicDevHosts.length > 0) {
    console.warn(
      `[vite] Vite dev server is configured for non-local browser access (${publicDevHosts.join(', ')}). This is acceptable for intentional dev tunnels/demo links, but it is not a production deployment path.`,
    );
  }
}

/**
 * @param {string | undefined} rawApiUrl
 */
function resolveApiProxyTarget(rawApiUrl) {
  const value = String(rawApiUrl || '').trim();
  if (!value) return null;

  try {
    const parsed = new URL(value);
    return parsed.origin;
  } catch {
    return null;
  }
}

// https://vite.dev/config/
// @ts-ignore
export default defineConfig(async ({ mode }) => {
  const publicFieldInjectPath = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    'src/lib/shims/esbuild-public-field.js',
  );
  // Load env file based on `mode` in the current working directory.
  // Set the third parameter to '' to load all env regardless of the `VITE_` prefix.
  const env = loadEnv(mode, process.cwd(), '');

  // Extract hostnames from CORS_ALLOWED_ORIGINS for allowedHosts
  const corsOrigins = csvList(env.CORS_ALLOWED_ORIGINS);
  const parsedHosts = corsOrigins
    .map((origin) => {
      try {
        return new URL(origin.trim()).hostname;
      } catch {
        return null;
      }
    })
    .filter(Boolean);

  const explicitAllowedHosts = csvList(env.VITE_ALLOWED_HOSTS);
  const apiProxyTarget = resolveApiProxyTarget(env.VITE_API_URL);
  const wildcardHostsEnabled = explicitAllowedHosts.includes('all');
  const publicDevHosts = collectPublicDevHosts(explicitAllowedHosts, corsOrigins, host);
  const allowUnsafePublicDev = env.ALLOW_UNSAFE_PUBLIC_DEV === '1';

  if (wildcardHostsEnabled && !allowUnsafePublicDev) {
    throw new Error(
      'Refusing to start Vite dev server with VITE_ALLOWED_HOSTS=all. If this is an intentional dev tunnel or remote debug session, set ALLOW_UNSAFE_PUBLIC_DEV=1 explicitly.',
    );
  }

  warnPublicDevExposure(publicDevHosts, wildcardHostsEnabled);

  // Combine all sources
  const finalAllowedHosts = [
    ...new Set([...parsedHosts, ...explicitAllowedHosts, 'localhost', '127.0.0.1']),
  ];

  return {
    plugins: [await sveltekit()],
    optimizeDeps: {
      esbuildOptions: {
        inject: [publicFieldInjectPath],
      },
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      allowedHosts: wildcardHostsEnabled ? true : finalAllowedHosts,
      proxy: apiProxyTarget
        ? {
            '/api': {
              target: apiProxyTarget,
              changeOrigin: true,
              secure: false,
              ws: true,
            },
          }
        : undefined,
      hmr: host
        ? {
            protocol: 'ws',
            host,
            port: 1421,
            clientPort: 1421,
          }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ['**/src-tauri/**'],
      },
      cors: true, // Enable CORS (or customize via VITE_CORS_ORIGIN if needed)
    },
  };
});

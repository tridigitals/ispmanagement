import { defineConfig, loadEnv } from "vite";
// @ts-nocheck
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  // Load env file based on `mode` in the current working directory.
  // Set the third parameter to '' to load all env regardless of the `VITE_` prefix.
  const env = loadEnv(mode, process.cwd(), '');

  // Extract hostnames from CORS_ALLOWED_ORIGINS for allowedHosts
  const corsOrigins = (env.CORS_ALLOWED_ORIGINS || "").split(",");
  const parsedHosts = corsOrigins.map(origin => {
    try {
      // Remove trailing slash if present before parsing (though URL ctor handles it)
      return new URL(origin.trim()).hostname;
    } catch {
      return null; // Ignore invalid URLs
    }
  }).filter(Boolean);

  const explicitAllowedHosts = (env.VITE_ALLOWED_HOSTS || "").split(",").filter(Boolean);

  // Combine all sources
  const finalAllowedHosts = [...new Set([...parsedHosts, ...explicitAllowedHosts, 'localhost', '127.0.0.1'])];

  return {
    plugins: [sveltekit()],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      allowedHosts: explicitAllowedHosts.includes('all') ? true : finalAllowedHosts,
      hmr: host
        ? {
          protocol: "ws",
          host,
          port: 1421,
          clientPort: 1421,
        }
        : undefined,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
      cors: true, // Enable CORS (or customize via VITE_CORS_ORIGIN if needed)
    },
  };
});

import { defineConfig, loadEnv } from "vite";
// @ts-nocheck
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  // Load env file based on `mode` in the current working directory.
  // Set the third parameter to '' to load all env regardless of the `VITE_` prefix.
  const env = loadEnv(mode, process.cwd(), '');

  const allowedHosts = (env.VITE_ALLOWED_HOSTS || "").split(",").filter(Boolean);

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
      allowedHosts: allowedHosts.includes('all') ? true : [...allowedHosts, 'localhost', '127.0.0.1', 'saas.tridigitals.com'],
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

import fs from 'fs';
import path from 'path';
import { URL } from 'url';

const envPath = path.resolve(process.cwd(), '.env');
const configPath = path.resolve(process.cwd(), 'src-tauri/tauri.conf.json');

function getEnvValue(content, key) {
  const regex = new RegExp(`^${key}=["']?([^"'\r\n]+)["']?`, 'm');
  const match = content.match(regex);
  return match ? match[1] : null;
}

function syncConfig() {
  try {
    if (!fs.existsSync(envPath)) {
      console.log('⚠ .env file not found, skipping sync.');
      return;
    }

    // Read .env
    const envContent = fs.readFileSync(envPath, 'utf8');
    const appName = getEnvValue(envContent, 'APP_NAME');
    const apiUrl = getEnvValue(envContent, 'VITE_API_URL');

    // Read tauri.conf.json
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    let changed = false;

    // Sync APP_NAME
    if (appName) {
      console.log(`ℹ Syncing APP_NAME: "${appName}"`);
      if (config.productName !== appName) {
        config.productName = appName;
        changed = true;
      }
      if (config.app?.windows?.[0]?.title !== appName) {
        if (!config.app) config.app = {};
        if (!config.app.windows) config.app.windows = [{}];
        config.app.windows[0].title = appName;
        changed = true;
      }
    }

    // Sync CSP based on VITE_API_URL
    if (apiUrl) {
      try {
        const url = new URL(apiUrl);
        const host = url.hostname;
        const wssUrl = `wss://${host}`;

        console.log(`ℹ Syncing CSP for host: "${host}"`);

        const baseCsp =
          "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data: asset: https:;";
        const connectSrcBase =
          "connect-src 'self' ipc: http://ipc.localhost http://localhost:3000 ws://localhost:3000 https:";

        // Construct new CSP with dynamic WSS URL
        const newCsp = `${baseCsp} ${connectSrcBase} ${wssUrl};`;

        if (config.app?.security?.csp !== newCsp) {
          if (!config.app) config.app = {};
          if (!config.app.security) config.app.security = {};

          config.app.security.csp = newCsp;
          changed = true;
        }
      } catch (e) {
        console.warn(`⚠ Could not parse VITE_API_URL: "${apiUrl}", skipping CSP sync.`);
      }
    }

    if (changed) {
      fs.writeFileSync(configPath, JSON.stringify(config, null, 2), 'utf8');
      console.log('✓ tauri.conf.json updated successfully.');
    } else {
      console.log('ℹ tauri.conf.json is already up to date.');
    }
  } catch (error) {
    console.error('✖ Error syncing config:', error.message);
  }
}

syncConfig();

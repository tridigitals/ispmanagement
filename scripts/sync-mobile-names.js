#!/usr/bin/env node
/**
 * sync-mobile-names.js — buat nama aplikasi mobile dinamis mengikuti env.
 *
 * Sumber nilai (prioritas): process.env > .env di repo root > default.
 *   APP_NAME_ADMIN      (default: ISP Admin)
 *   APP_NAME_CUSTOMER   (default: ISP Customer)
 *   APP_NAME_TECHNICIAN (default: ISP Teknisi)
 *
 * Target yang di-patch (build-time; launcher label tidak bisa berubah runtime):
 *   apps/<app>/android/app/src/main/AndroidManifest.xml  -> android:label
 *   apps/<app>/ios/Runner/Info.plist                     -> CFBundleDisplayName (bila ada)
 *
 * Nilai yang sama dilempar ke Dart lewat --dart-define=APP_NAME oleh build script
 * (lihat build-apk.sh masing-masing app) — dibaca app_config.dart via
 * String.fromEnvironment.
 *
 * Usage:
 *   node scripts/sync-mobile-names.js [app...]   # patch semua/tertentu
 *   node scripts/sync-mobile-names.js --get APP_NAME_CUSTOMER   # print nilai resolved
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const ENV_PATH = path.join(ROOT, '.env');

const APPS = {
  'mobile-admin': { envKey: 'APP_NAME_ADMIN', default: 'ISP Admin' },
  'mobile-customer': { envKey: 'APP_NAME_CUSTOMER', default: 'ISP Customer' },
  'mobile-technician': { envKey: 'APP_NAME_TECHNICIAN', default: 'ISP Teknisi' },
};

function readDotEnv() {
  if (!fs.existsSync(ENV_PATH)) return {};
  const out = {};
  for (const line of fs.readFileSync(ENV_PATH, 'utf8').split(/\r?\n/)) {
    const m = line.match(/^([A-Z0-9_]+)=["']?([^"'\r\n]*)["']?\s*$/);
    if (m) out[m[1]] = m[2];
  }
  return out;
}

function resolveName(envKey, fallback) {
  if (process.env[envKey]) return process.env[envKey];
  const dot = readDotEnv();
  if (dot[envKey]) return dot[envKey];
  return fallback;
}

function xmlEscape(v) {
  return v.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function patchFile(label, file, regex, replacer) {
  if (!fs.existsSync(file)) {
    console.log(`   – ${label}: tidak ada, skip (${path.relative(ROOT, file)})`);
    return false;
  }
  const before = fs.readFileSync(file, 'utf8');
  const after = before.replace(regex, replacer);
  if (after === before) {
    console.log(`   ✓ ${label}: sudah sesuai`);
    return false;
  }
  fs.writeFileSync(file, after, 'utf8');
  console.log(`   ✎ ${label}: dipatch -> ${path.relative(ROOT, file)}`);
  return true;
}

function syncApp(name) {
  const cfg = APPS[name];
  const value = resolveName(cfg.envKey, cfg.default);
  console.log(`\n▶ ${name}  (${cfg.envKey} = "${value}")`);
  patchFile(
    'android:label',
    path.join(ROOT, 'apps', name, 'android/app/src/main/AndroidManifest.xml'),
    /(android:label=")[^"]*(")/,
    (_, a, b) => a + xmlEscape(value) + b,
  );
  patchFile(
    'CFBundleDisplayName',
    path.join(ROOT, 'apps', name, 'ios/Runner/Info.plist'),
    /(<key>CFBundleDisplayName<\/key>\s*<string>)[^<]*(<\/string>)/,
    (_, a, b) => a + value + b,
  );
  return value;
}

const args = process.argv.slice(2);
if (args[0] === '--get') {
  const key = args[1];
  const entry = Object.values(APPS).find((a) => a.envKey === key);
  if (!entry) {
    console.error(`unknown key ${key}`);
    process.exit(1);
  }
  process.stdout.write(resolveName(entry.envKey, entry.default));
  process.exit(0);
}

const targets = args.length ? args : Object.keys(APPS);
for (const t of targets) {
  if (!APPS[t]) {
    console.error(`app tidak dikenal: ${t} (pilihan: ${Object.keys(APPS).join(', ')})`);
    process.exit(1);
  }
  syncApp(t);
}
console.log('\n✓ sync-mobile-names selesai');

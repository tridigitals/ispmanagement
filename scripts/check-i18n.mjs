import fs from 'node:fs';
import path from 'node:path';

const repoRoot = process.cwd();

const LOCALES = [
  { code: 'en', file: 'src/lib/i18n/locales/en.json' },
  { code: 'id', file: 'src/lib/i18n/locales/id.json' },
];

function walk(dir) {
  const out = [];
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out;
}

function flattenKeys(obj, prefix = '') {
  const keys = new Set();
  if (obj && typeof obj === 'object' && !Array.isArray(obj)) {
    for (const [k, v] of Object.entries(obj)) {
      const next = prefix ? `${prefix}.${k}` : k;
      keys.add(next);
      for (const nk of flattenKeys(v, next)) keys.add(nk);
    }
  }
  return keys;
}

function readJson(rel) {
  const abs = path.join(repoRoot, rel);
  return JSON.parse(fs.readFileSync(abs, 'utf8'));
}

function extractKeys(text) {
  const keys = new Set();
  const patterns = [
    /\$t\(\s*'([^']+)'\s*(?:,|\))/g,
    /\$t\(\s*"([^"]+)"\s*(?:,|\))/g,
    /\bt\(\s*'([^']+)'\s*(?:,|\))/g,
    /\bt\(\s*"([^"]+)"\s*(?:,|\))/g,
  ];
  for (const re of patterns) {
    let m;
    while ((m = re.exec(text))) keys.add(m[1]);
  }
  return keys;
}

function parseArgs(argv) {
  const args = new Set(argv);
  return {
    json: args.has('--json'),
    failOnMissing: !args.has('--no-fail'),
    showUnused: args.has('--show-unused'),
  };
}

const opts = parseArgs(process.argv.slice(2));
const sourceFiles = walk(path.join(repoRoot, 'src')).filter((p) => /\.(svelte|ts|js)$/.test(p));

const usedKeys = new Set();
for (const file of sourceFiles) {
  const txt = fs.readFileSync(file, 'utf8');
  for (const k of extractKeys(txt)) usedKeys.add(k);
}

const localeKeysets = new Map();
for (const loc of LOCALES) {
  const json = readJson(loc.file);
  localeKeysets.set(loc.code, flattenKeys(json));
}

// Find keys used in code that are missing from locale files.
const results = [];
for (const loc of LOCALES) {
  const have = localeKeysets.get(loc.code);
  const missing = [...usedKeys].filter((k) => !have.has(k)).sort();
  results.push({ locale: loc.code, missing });
}

// Also find orphan keys (present but unused) - optional, keep for info.
const totalMissing = results.reduce((n, r) => n + r.missing.length, 0);

if (opts.json) {
  const unusedByLocale = [];
  for (const loc of LOCALES) {
    const have = localeKeysets.get(loc.code);
    const unused = [...have].filter((k) => !usedKeys.has(k)).sort();
    unusedByLocale.push({ locale: loc.code, unusedCount: unused.length });
  }
  console.log(JSON.stringify({ usedCount: usedKeys.size, results, unusedByLocale }, null, 2));
} else {
  console.log(`i18n keys used in source: ${usedKeys.size}`);
  for (const r of results) {
    console.log(`- ${r.locale}: ${r.missing.length} missing key(s)`);
    for (const k of r.missing.slice(0, 40)) console.log(`  • ${k}`);
    if (r.missing.length > 40) console.log(`  ... and ${r.missing.length - 40} more`);
  }

  if (opts.showUnused) {
    for (const loc of LOCALES) {
      const have = localeKeysets.get(loc.code);
      const unused = [...have].filter((k) => !usedKeys.has(k)).sort();
      console.log(`- ${loc.code}: ${unused.length} unused key(s)`);
    }
  }
}

if (opts.failOnMissing && totalMissing > 0) {
  process.exitCode = 1;
}

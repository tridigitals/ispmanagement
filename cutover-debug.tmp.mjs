import { chromium } from 'playwright';

const BASE = 'http://127.0.0.1:1420';
const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();

await page.goto(BASE + '/login', { waitUntil: 'networkidle' });
console.log('form fields:', await page.locator('input').count());
const idSel = await page.locator('#identifier').count() ? '#identifier' : '#login-email';
const pwSel = await page.locator('#password').count() ? '#password' : '#login-password';
console.log('selectors:', idSel, pwSel);
await page.fill(idSel, process.env.E2E_USER);
await page.fill(pwSel, process.env.E2E_PASS);
await page.click('button[type=submit]');
await page.waitForTimeout(6000);
console.log('URL after submit =', page.url());
const body = await page.locator('body').innerText();
console.log('BODY SNIPPET:', body.slice(0, 400).replace(/\n+/g, ' | '));
await page.screenshot({ path: 'sketches/redesign-2026-09/ss-cutover-debug.png' });
await browser.close();

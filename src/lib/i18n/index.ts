import { register, init, getLocaleFromNavigator } from 'svelte-i18n';

register('en', () => import('./locales/en.json'));
register('en-US', () => import('./locales/en.json'));
register('id', () => import('./locales/id.json'));
register('id-ID', () => import('./locales/id.json'));

const browserLocale = getLocaleFromNavigator();
const normalizedLocale = browserLocale ? browserLocale.replace('_', '-') : 'en';

init({
  fallbackLocale: 'en',
  initialLocale: normalizedLocale || 'en',
});

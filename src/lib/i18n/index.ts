import { addMessages, getLocaleFromNavigator, init, locale } from 'svelte-i18n';

import {
  I18N_ALL_NAMESPACES,
  normalizeAppLocale,
  resolveBootNamespaces,
  type AppLocale,
  type I18nNamespace,
} from './i18nBootPlan';

const localeNamespaceModules = import.meta.glob('./namespaces/*/*.json');
const loadedNamespacePromises = new Map<string, Promise<void>>();

type LocaleDictionaryValue =
  | string
  | null
  | LocaleDictionaryValue[]
  | { [key: string]: LocaleDictionaryValue };

const browserLocale = getLocaleFromNavigator();
const initialLocale = normalizeAppLocale(browserLocale);

init({
  fallbackLocale: 'en',
  initialLocale,
});

function namespaceModuleKey(localeCode: AppLocale, namespace: I18nNamespace) {
  return `./namespaces/${localeCode}/${namespace}.json`;
}

async function loadLocaleNamespace(localeCode: AppLocale, namespace: I18nNamespace) {
  const cacheKey = `${localeCode}:${namespace}`;
  if (!loadedNamespacePromises.has(cacheKey)) {
    loadedNamespacePromises.set(
      cacheKey,
      (async () => {
        const moduleKey = namespaceModuleKey(localeCode, namespace);
        const loader = localeNamespaceModules[moduleKey];
        if (!loader) {
          throw new Error(`Missing i18n namespace module: ${moduleKey}`);
        }
        const loaded = (await loader()) as { default?: LocaleDictionaryValue };
        const namespaceMessages = (loaded.default ?? {}) as LocaleDictionaryValue;
        addMessages(
          localeCode,
          {
            [namespace]: namespaceMessages,
          } as any,
        );
      })(),
    );
  }

  await loadedNamespacePromises.get(cacheKey);
}

export async function ensureLocaleNamespaces(
  inputLocale: string | null | undefined,
  namespaces: readonly I18nNamespace[],
): Promise<AppLocale> {
  const normalizedLocale = normalizeAppLocale(inputLocale);
  await Promise.all(namespaces.map((namespace) => loadLocaleNamespace(normalizedLocale, namespace)));
  locale.set(normalizedLocale);
  return normalizedLocale;
}

export async function ensureBootLocale(
  pathname: string,
  inputLocale?: string | null,
): Promise<AppLocale> {
  return ensureLocaleNamespaces(inputLocale, resolveBootNamespaces(pathname));
}

export async function ensureFullLocale(inputLocale?: string | null): Promise<AppLocale> {
  return ensureLocaleNamespaces(inputLocale, I18N_ALL_NAMESPACES);
}

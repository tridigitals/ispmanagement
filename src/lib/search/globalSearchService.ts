import { getGlobalSearchProviders } from './globalSearchProviders';
import {
  groupGlobalSearchResults,
  type GlobalSearchProvider,
  type GlobalSearchProviderContext,
  type GlobalSearchResult,
  type GlobalSearchResultGroup,
} from './globalSearchModel';

function scoreMatch(query: string, item: GlobalSearchResult): number {
  const needle = query.trim().toLowerCase();
  const title = item.title.toLowerCase();
  const subtitle = item.subtitle.toLowerCase();

  if (title === needle) return 400;
  if (title.startsWith(needle)) return 300;
  if (title.includes(needle)) return 200;
  if (subtitle.startsWith(needle)) return 120;
  if (subtitle.includes(needle)) return 80;
  return 0;
}

function rankItems(query: string, items: GlobalSearchResult[]): GlobalSearchResult[] {
  return [...items].sort((left, right) => {
    const scoreDiff = scoreMatch(query, right) - scoreMatch(query, left);
    if (scoreDiff !== 0) return scoreDiff;
    return left.title.localeCompare(right.title);
  });
}

export async function searchGlobalTopbar(
  rawQuery: string,
  context: GlobalSearchProviderContext,
  providers: GlobalSearchProvider[] = getGlobalSearchProviders(),
): Promise<{ query: string; groups: GlobalSearchResultGroup[] }> {
  const query = rawQuery.trim();
  if (!query) {
    return { query, groups: [] };
  }

  const enabledProviders = providers.filter((provider) => provider.isEnabled(context));
  const settled = await Promise.all(
    enabledProviders.map(async (provider) => {
      if (query.length < (provider.minQueryLength ?? 1)) {
        return { key: provider.key, items: [] };
      }
      try {
        const items = await provider.search(query, context);
        return { key: provider.key, items: rankItems(query, items) };
      } catch {
        return { key: provider.key, items: [] };
      }
    }),
  );

  const allItems = settled.flatMap((entry) => entry.items);
  return {
    query,
    groups: groupGlobalSearchResults(
      allItems,
      enabledProviders.map((provider) => provider.key),
    ),
  };
}

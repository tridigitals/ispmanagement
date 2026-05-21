export type GlobalSearchResultKind =
  | 'customer'
  | 'router'
  | 'invoice'
  | 'support-ticket'
  | 'team-member'
  | 'tenant';

export interface GlobalSearchResult {
  id: string;
  kind: GlobalSearchResultKind;
  title: string;
  subtitle: string;
  href: string;
  groupKey: string;
  groupLabel: string;
}

export interface GlobalSearchResultGroup {
  key: string;
  label: string;
  items: GlobalSearchResult[];
}

export interface GlobalSearchProviderContext {
  can: (action: string, resource: string) => boolean;
  isSuperAdmin: boolean;
  shellScope: 'admin' | 'superadmin' | 'workspace';
  tenantPrefix: string;
}

export interface GlobalSearchProvider {
  key: string;
  label: string;
  isEnabled: (context: GlobalSearchProviderContext) => boolean;
  minQueryLength?: number;
  search: (
    query: string,
    context: GlobalSearchProviderContext,
  ) => Promise<GlobalSearchResult[]>;
}

export function groupGlobalSearchResults(
  results: GlobalSearchResult[],
  providerOrder: string[],
): GlobalSearchResultGroup[] {
  const grouped = new Map<string, GlobalSearchResultGroup>();

  for (const result of results) {
    const existing = grouped.get(result.groupKey);
    if (existing) {
      existing.items.push(result);
      continue;
    }
    grouped.set(result.groupKey, {
      key: result.groupKey,
      label: result.groupLabel,
      items: [result],
    });
  }

  return providerOrder
    .map((key) => grouped.get(key))
    .filter((group): group is GlobalSearchResultGroup => Boolean(group));
}

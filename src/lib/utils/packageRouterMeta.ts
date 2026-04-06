type RouterMirrorNameRow = {
  id?: string | null;
  name?: string | null;
  router_present?: boolean | null;
};

function normalizeText(value: string | null | undefined): string {
  return String(value ?? '').trim();
}

export function getAvailableRouterNameSuggestions(rows: RouterMirrorNameRow[]): string[] {
  const names = rows
    .filter((row) => Boolean(row?.router_present))
    .map((row) => normalizeText(row?.name))
    .filter(Boolean);

  return Array.from(new Set(names)).sort((left, right) => left.localeCompare(right));
}

type NamedSuggestion = { id?: string; name: string };

function normalizeText(value: string | null | undefined): string {
  return String(value ?? '').trim();
}

function hasSuggestion(name: string, suggestions: NamedSuggestion[]): boolean {
  const target = normalizeText(name);
  if (!target) return true;
  return suggestions.some((item) => normalizeText(item.name) === target);
}

export function getPackageRouterMappingReferenceError(args: {
  routerId: string | null | undefined;
  profileName: string | null | undefined;
  profileSuggestions: NamedSuggestion[];
  poolName: string | null | undefined;
  poolSuggestions: NamedSuggestion[];
}): string | null {
  const routerId = normalizeText(args.routerId);
  const profileName = normalizeText(args.profileName);
  const poolName = normalizeText(args.poolName);

  if (!routerId || !profileName) return null;

  if (!hasSuggestion(profileName, args.profileSuggestions)) {
    return `Selected PPP profile '${profileName}' is no longer available on this router. Sync PPP profiles and choose a valid profile.`;
  }

  if (poolName && !hasSuggestion(poolName, args.poolSuggestions)) {
    return `Selected IP pool '${poolName}' is no longer available on this router. Sync IP pools and choose a valid pool.`;
  }

  return null;
}

export function getPackageRouterMappingErrorFallback(message: string | null | undefined): string {
  const text = normalizeText(message);
  if (!text) return 'Failed to save router mapping.';

  if (text.includes('Selected PPP profile does not exist on this router')) {
    return 'The selected PPP profile is no longer available on this router. Sync PPP profiles and choose another profile.';
  }

  if (text.includes('Selected IP pool') && text.includes('does not exist on this router')) {
    return 'The selected IP pool is no longer available on this router. Sync IP pools and choose another pool.';
  }

  return text;
}

export function normalizeLegacyBasePath(pathname: string, search = ''): string | null {
  if (pathname !== '/isp-management' && !pathname.startsWith('/isp-management/')) {
    return null;
  }

  const cleanPath = pathname.replace(/^\/isp-management/, '') || '/';
  return `${cleanPath}${search}`;
}

export function isLocalHostname(hostname: string): boolean {
  return (
    hostname.includes('localhost') || hostname.includes('127.0.0.1') || hostname.includes('tauri')
  );
}

export function shouldLookupCustomDomain(args: {
  hostname: string;
  knownSlug: string | null | undefined;
  isPlatformDomain: boolean;
}): boolean {
  return !args.knownSlug && !isLocalHostname(args.hostname) && !args.isPlatformDomain;
}

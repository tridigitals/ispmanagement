export function shouldShowManualEndpointSection(activeAssetConnectSourceId: string | null) {
  return String(activeAssetConnectSourceId || '').trim().length === 0;
}

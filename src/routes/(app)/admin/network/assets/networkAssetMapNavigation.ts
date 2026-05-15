export function buildNetworkAssetMapUrl(args: {
  tenantPrefix: string;
  assetId: string;
  latitude: number;
  longitude: number;
}) {
  const params = new URLSearchParams({
    asset_id: args.assetId,
    asset_lat: String(args.latitude),
    asset_lng: String(args.longitude),
  });

  return `${args.tenantPrefix}/admin/network/map?${params.toString()}`;
}

export function parseNetworkAssetMapTarget(searchParams: URLSearchParams): {
  assetId: string;
  latitude: number;
  longitude: number;
} | null {
  const assetId = String(searchParams.get('asset_id') || '').trim();
  const latitude = Number(searchParams.get('asset_lat'));
  const longitude = Number(searchParams.get('asset_lng'));

  if (!assetId) return null;
  if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) return null;

  return {
    assetId,
    latitude,
    longitude,
  };
}

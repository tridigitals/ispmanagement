export function parseNetworkAssetCoordinates(latitudeRaw: string, longitudeRaw: string): {
  latitude: number | null;
  longitude: number | null;
  error: 'pair' | 'latitude_range' | 'longitude_range' | 'invalid' | null;
} {
  const latText = latitudeRaw.trim();
  const lngText = longitudeRaw.trim();

  if (!latText && !lngText) {
    return { latitude: null, longitude: null, error: null };
  }

  if (!latText || !lngText) {
    return { latitude: null, longitude: null, error: 'pair' };
  }

  const latitude = Number(latText);
  const longitude = Number(lngText);
  if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) {
    return { latitude: null, longitude: null, error: 'invalid' };
  }
  if (latitude < -90 || latitude > 90) {
    return { latitude: null, longitude: null, error: 'latitude_range' };
  }
  if (longitude < -180 || longitude > 180) {
    return { latitude: null, longitude: null, error: 'longitude_range' };
  }

  return { latitude, longitude, error: null };
}

export function copyCustomerLocationCoordinates(location: {
  latitude?: number | null;
  longitude?: number | null;
} | null): { latitude: string; longitude: string } | null {
  if (location?.latitude == null || location?.longitude == null) return null;
  return {
    latitude: String(location.latitude),
    longitude: String(location.longitude),
  };
}

export function formatNetworkAssetCoordinates(
  latitude: number | null | undefined,
  longitude: number | null | undefined,
): string {
  if (latitude == null || longitude == null) return '';
  return `${Number(latitude).toFixed(6)}, ${Number(longitude).toFixed(6)}`;
}

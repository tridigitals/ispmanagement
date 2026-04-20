export function parseOptionalCoordinateInput(value: string): number | null {
  const raw = value.trim();
  if (!raw) return null;
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? parsed : Number.NaN;
}

export function validateOptionalCoordinates(
  latitudeInput: string,
  longitudeInput: string,
): { latitude: number | null; longitude: number | null; error: string | null } {
  const latitude = parseOptionalCoordinateInput(latitudeInput);
  const longitude = parseOptionalCoordinateInput(longitudeInput);

  if ((latitude == null) !== (longitude == null)) {
    return {
      latitude: null,
      longitude: null,
      error: 'both_required',
    };
  }
  if ((latitude != null && !Number.isFinite(latitude)) || (longitude != null && !Number.isFinite(longitude))) {
    return {
      latitude: null,
      longitude: null,
      error: 'invalid_number',
    };
  }
  if (latitude != null && (latitude < -90 || latitude > 90)) {
    return {
      latitude: null,
      longitude: null,
      error: 'latitude_range',
    };
  }
  if (longitude != null && (longitude < -180 || longitude > 180)) {
    return {
      latitude: null,
      longitude: null,
      error: 'longitude_range',
    };
  }

  return {
    latitude,
    longitude,
    error: null,
  };
}

export function formatLocationCoordinates(
  latitude: number | null | undefined,
  longitude: number | null | undefined,
): string | null {
  if (latitude == null || longitude == null) return null;
  if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) return null;
  return `${Number(latitude).toFixed(6)}, ${Number(longitude).toFixed(6)}`;
}

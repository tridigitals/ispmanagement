type MapFeatureLike = {
  properties?: Record<string, any>;
  geometry?: Record<string, any> | null;
};

type MapFeatureSnapshot = {
  feature: MapFeatureLike;
  properties: Record<string, any>;
};

function cloneSerializable<T>(value: T): T {
  return JSON.parse(JSON.stringify(value));
}

export function snapshotMapFeature(feature: MapFeatureLike | null | undefined): MapFeatureSnapshot | null {
  if (!feature) return null;

  const clonedFeature = cloneSerializable({
    properties: feature.properties || {},
    geometry: feature.geometry || null,
  });

  return {
    feature: clonedFeature,
    properties: clonedFeature.properties || {},
  };
}

import { describe, expect, it } from 'vitest';

import { snapshotMapFeature } from './networkMapEventSnapshot';

describe('network map event snapshot', () => {
  it('preserves feature data even if the original event payload changes later', () => {
    const feature = {
      properties: {
        id: 'node-1',
        name: 'Core POP',
      },
      geometry: {
        type: 'Point',
        coordinates: [106.8, -6.2],
      },
    };

    const snapshot = snapshotMapFeature(feature);

    feature.properties.id = 'mutated';
    feature.geometry.coordinates[0] = 0;

    expect(snapshot).toEqual({
      feature: {
        properties: {
          id: 'node-1',
          name: 'Core POP',
        },
        geometry: {
          type: 'Point',
          coordinates: [106.8, -6.2],
        },
      },
      properties: {
        id: 'node-1',
        name: 'Core POP',
      },
    });
  });

  it('returns null when there is no feature to snapshot', () => {
    expect(snapshotMapFeature(null)).toBeNull();
  });
});

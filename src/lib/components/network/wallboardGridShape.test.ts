import { describe, expect, it } from 'vitest';
import { gridShapeFor } from './wallboardPageHelpers';

describe('gridShapeFor', () => {
  it('kosong -> 1x1', () => {
    expect(gridShapeFor(0, 4, 3)).toEqual({ cols: 1, rows: 1 });
  });

  it('tile sedikit tidak memaksa 4 kolom', () => {
    expect(gridShapeFor(1, 4, 3)).toEqual({ cols: 1, rows: 1 });
    expect(gridShapeFor(2, 4, 3)).toEqual({ cols: 2, rows: 1 });
    expect(gridShapeFor(3, 4, 3)).toEqual({ cols: 3, rows: 1 });
    expect(gridShapeFor(4, 4, 3)).toEqual({ cols: 4, rows: 1 });
  });

  it('5 tile @4x3 -> 3x2 (sel kosong minimal, tie-break kolom besar)', () => {
    expect(gridShapeFor(5, 4, 3)).toEqual({ cols: 3, rows: 2 });
  });

  it('penuh persis tetap preset', () => {
    expect(gridShapeFor(12, 4, 3)).toEqual({ cols: 4, rows: 3 });
    expect(gridShapeFor(9, 4, 3)).toEqual({ cols: 3, rows: 3 });
  });

  it('baris tidak pernah melebihi maxRows', () => {
    expect(gridShapeFor(10, 4, 3).rows).toBeLessThanOrEqual(3);
    expect(gridShapeFor(6, 3, 2)).toEqual({ cols: 3, rows: 2 });
  });
});

export type ThresholdUnit = 'Kbps' | 'Mbps' | 'Gbps';

export type ThresholdInput = {
  value: string;
  unit: ThresholdUnit;
};

export function thresholdInputFromBps(bps: number | null | undefined): ThresholdInput {
  if (bps == null || !Number.isFinite(bps) || bps <= 0) {
    return { value: '', unit: 'Kbps' };
  }

  if (bps >= 1_000_000_000) {
    return {
      unit: 'Gbps',
      value: String((bps / 1_000_000_000).toFixed(3).replace(/\.?0+$/, '')),
    };
  }
  if (bps >= 1_000_000) {
    return {
      unit: 'Mbps',
      value: String((bps / 1_000_000).toFixed(3).replace(/\.?0+$/, '')),
    };
  }
  return {
    unit: 'Kbps',
    value: String((bps / 1_000).toFixed(3).replace(/\.?0+$/, '')),
  };
}

export function thresholdBpsFromInput(value: string, unit: ThresholdUnit): number | null {
  const num = Number.parseFloat(value || '');
  if (!Number.isFinite(num) || num <= 0) return null;

  const mul = unit === 'Gbps' ? 1_000_000_000 : unit === 'Mbps' ? 1_000_000 : 1_000;
  return Math.round(num * mul);
}

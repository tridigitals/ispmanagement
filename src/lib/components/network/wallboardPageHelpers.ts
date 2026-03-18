import { slotCountForLayout, type TrendInfo } from '$lib/components/network/wallboardUtils';
import type { LayoutPreset } from '$lib/constants/wallboard';

type RouterDisplayLike = {
  id: string;
  name: string;
  identity?: string | null;
  ros_version?: string | null;
};

export function formatWallboardBps(
  bps: number | null | undefined,
  formatter: (value: number | null | undefined, naLabel: string) => string,
  naLabel: string,
) {
  return formatter(bps, naLabel);
}

export function formatWallboardLatency(
  ms: number | null | undefined,
  formatter: (value: number | null | undefined, naLabel: string) => string,
  naLabel: string,
) {
  return formatter(ms, naLabel);
}

export function routerTitle(router: RouterDisplayLike) {
  const name = router.identity || router.name;
  const ros = router.ros_version ? ` • ROS ${router.ros_version}` : '';
  return `${name}${ros}`;
}

export function trendBadgeText(
  trend: TrendInfo,
  formatter: (trend: TrendInfo, stableLabel: string) => string,
  stableLabel: string,
) {
  return formatter(trend, stableLabel);
}

export function trendLabel(
  trend: TrendInfo,
  formatter: (trend: TrendInfo, labels: { up: string; down: string; stable: string }) => string,
  labels: { up: string; down: string; stable: string },
) {
  return formatter(trend, labels);
}

export function ensureSlotsForLayout<T>(slots: (T | null)[], layout: LayoutPreset) {
  const want = slotCountForLayout(layout);
  if (slots.length >= want) return slots;
  return [...slots, ...Array.from({ length: want - slots.length }, () => null)];
}

export function ensureSlotIndex<T>(slots: (T | null)[], idx: number) {
  if (idx < slots.length) return slots;
  return [...slots, ...Array.from({ length: idx + 1 - slots.length }, () => null)];
}

export function globalSlotIndex(page: number, layout: LayoutPreset, localIdx: number) {
  return page * slotCountForLayout(layout) + localIdx;
}

export function routerLabel<T extends { id: string; name: string; identity?: string | null }>(
  rows: T[],
  routerId: string,
) {
  const router = rows.find((row) => row.id === routerId) || null;
  return router?.identity || router?.name || routerId;
}

export function bucketLabel(
  bucket: 'raw' | 'hour' | 'day',
  labels: { raw: string; hour: string; day: string },
) {
  if (bucket === 'raw') return labels.raw;
  if (bucket === 'hour') return labels.hour;
  return labels.day;
}

export function bucketHint(
  bucket: 'raw' | 'hour' | 'day',
  labels: { raw: string; hour: string; day: string },
) {
  if (bucket === 'raw') return labels.raw;
  if (bucket === 'hour') return labels.hour;
  return labels.day;
}

export function buildMetricsCsvRows(
  rows: { ts: string; rx_bps: number; tx_bps: number }[],
  iface: string,
  routerName: string,
  bucket: 'raw' | 'hour' | 'day',
) {
  return rows.map((row) => ({
    timestamp: row.ts,
    router: routerName,
    interface: iface,
    bucket,
    rx_bps: row.rx_bps ?? 0,
    tx_bps: row.tx_bps ?? 0,
  }));
}

export function metricsCsvFilePrefix(iface: string, bucket: 'raw' | 'hour' | 'day') {
  return `metrics-${iface || 'interface'}-${bucket}`;
}

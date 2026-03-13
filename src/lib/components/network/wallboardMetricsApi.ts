export type WallboardMetricsSlotRef = {
  routerId: string;
  iface: string;
};

export function extractMetricsRows(payload: any): any[] {
  return Array.isArray(payload)
    ? payload
    : Array.isArray(payload?.items)
      ? payload.items
      : Array.isArray(payload?.data)
        ? payload.data
        : Array.isArray(payload?.rows)
          ? payload.rows
          : Array.isArray(payload?.history)
            ? payload.history
            : Array.isArray(payload?.metrics)
              ? payload.metrics
              : Array.isArray(payload?.result?.items)
                ? payload.result.items
                : Array.isArray(payload?.result?.data)
                  ? payload.result.data
                  : [];
}

function normalizeIfaceName(v: string): string {
  return String(v || '')
    .replace(/[\u200B-\u200D\uFEFF]/g, '')
    .trim()
    .toLowerCase()
    .replace(/\s+/g, ' ');
}

function canonicalIfaceName(v: string): string {
  return normalizeIfaceName(v).replace(/[^a-z0-9]+/g, '');
}

function rowInterfaceName(row: any): string {
  return normalizeIfaceName(
    String(
      row?.interface ??
        row?.interface_name ??
        row?.iface ??
        row?.name ??
        row?.interface_display_name ??
        '',
    ),
  );
}

function matchesIfaceName(rowName: string, target: string): boolean {
  if (!rowName || !target) return false;
  if (rowName === target) return true;
  return canonicalIfaceName(rowName) === canonicalIfaceName(target);
}

function filterRowsForIface(rows: any[], iface: string): { rows: any[]; hasIfaceDimension: boolean } {
  const target = normalizeIfaceName(String(iface || ''));
  if (!target) return { rows, hasIfaceDimension: false };

  const withIface = rows.filter((row) => rowInterfaceName(row));
  if (!withIface.length) return { rows, hasIfaceDimension: false };

  const strict = withIface.filter((row) => matchesIfaceName(rowInterfaceName(row), target));
  return { rows: strict, hasIfaceDimension: true };
}

export async function fetchInterfaceMetricsRows(args: {
  slot: WallboardMetricsSlotRef;
  minLimit: number;
  fetchMetrics: (
    routerId: string,
    params?: { interface?: string; limit?: number },
  ) => Promise<any>;
}) {
  const payload = await args.fetchMetrics(args.slot.routerId, {
    interface: args.slot.iface,
    limit: args.minLimit,
  });
  let rows = extractMetricsRows(payload);
  if (args.slot.iface) {
    const filtered = filterRowsForIface(rows, args.slot.iface);
    if (filtered.hasIfaceDimension) rows = filtered.rows;
  }

  return rows;
}

const DEFAULT_QUEUE_PRESETS = ['10M/10M', '20M/20M', '30M/30M', '50M/50M', '100M/100M'];
const MBPS_PATTERN = /(?:^|[^0-9])(\d{1,4})(?:\s*)(?:mbps|mb|m)(?:[^a-z]|$)/i;

type PackageBandwidthSource = {
  name?: string | null;
  description?: string | null;
  features?: string[] | null;
};

function extractMbpsFromText(value: string | null | undefined): number | null {
  if (!value) return null;
  const match = value.match(MBPS_PATTERN);
  if (!match?.[1]) return null;
  const parsed = Number(match[1]);
  if (!Number.isInteger(parsed) || parsed <= 0) return null;
  return parsed;
}

export function extractDhcpStaticPackageBandwidthMbps(
  source: PackageBandwidthSource,
): number | null {
  const fromName = extractMbpsFromText(source.name);
  if (fromName) return fromName;

  const fromDescription = extractMbpsFromText(source.description);
  if (fromDescription) return fromDescription;

  for (const feature of source.features || []) {
    const fromFeature = extractMbpsFromText(feature);
    if (fromFeature) return fromFeature;
  }

  return null;
}

export function buildDhcpStaticQueueRateLimitPresets(
  source: PackageBandwidthSource,
): string[] {
  const bandwidthMbps = extractDhcpStaticPackageBandwidthMbps(source);
  if (!bandwidthMbps) return [...DEFAULT_QUEUE_PRESETS];

  const derived = `${bandwidthMbps}M/${bandwidthMbps}M`;
  return [derived, ...DEFAULT_QUEUE_PRESETS.filter((preset) => preset !== derived)];
}

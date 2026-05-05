const MAC_COMPACT_PATTERN = /^[0-9a-f]{12}$/i;
const IPV4_PATTERN = /^(?:\d{1,3}\.){3}\d{1,3}$/;
const QUEUE_RATE_LIMIT_PATTERN =
  /^\s*\d+(?:\.\d+)?[kKmMgG]?(?:\/\d+(?:\.\d+)?[kKmMgG]?)\s*$/;

export function formatDhcpStaticMacAddressInput(value: string): string {
  const compact = value.replace(/[^0-9a-f]/gi, '').toUpperCase().slice(0, 12);
  const groups = compact.match(/.{1,2}/g) || [];
  return groups.join(':');
}

export function normalizeDhcpStaticMacAddress(
  value: string,
): { value: string | null; error: 'invalid_mac' | null } {
  const compact = value.replace(/[^0-9a-f]/gi, '').toUpperCase();
  if (!MAC_COMPACT_PATTERN.test(compact)) {
    return { value: null, error: 'invalid_mac' };
  }

  const groups = compact.match(/.{1,2}/g) || [];
  return {
    value: groups.join(':'),
    error: null,
  };
}

export function validateDhcpStaticIpv4Address(value: string): 'invalid_ip' | null {
  const trimmed = value.trim();
  if (!IPV4_PATTERN.test(trimmed)) return 'invalid_ip';

  const octets = trimmed.split('.').map((part) => Number(part));
  if (octets.some((octet) => !Number.isInteger(octet) || octet < 0 || octet > 255)) {
    return 'invalid_ip';
  }

  return null;
}

export function validateDhcpStaticQueueRateLimit(
  value: string,
): 'invalid_queue_rate' | null {
  const trimmed = value.trim();
  if (!trimmed) return 'invalid_queue_rate';
  return QUEUE_RATE_LIMIT_PATTERN.test(trimmed) ? null : 'invalid_queue_rate';
}

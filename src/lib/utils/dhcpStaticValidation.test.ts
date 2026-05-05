import { describe, expect, it } from 'vitest';
import {
  formatDhcpStaticMacAddressInput,
  normalizeDhcpStaticMacAddress,
  validateDhcpStaticIpv4Address,
  validateDhcpStaticQueueRateLimit,
} from './dhcpStaticValidation';

describe('dhcpStaticValidation', () => {
  it('formats partial MAC input while typing', () => {
    expect(formatDhcpStaticMacAddressInput('aabbcc')).toBe('AA:BB:CC');
    expect(formatDhcpStaticMacAddressInput('aa-bb-cc-dd-ee-ff-11')).toBe('AA:BB:CC:DD:EE:FF');
  });

  it('normalizes MAC address into uppercase colon-delimited format', () => {
    expect(normalizeDhcpStaticMacAddress('aa-bb-cc-dd-ee-ff')).toEqual({
      value: 'AA:BB:CC:DD:EE:FF',
      error: null,
    });
  });

  it('rejects malformed MAC address', () => {
    expect(normalizeDhcpStaticMacAddress('aa:bb:cc')).toEqual({
      value: null,
      error: 'invalid_mac',
    });
  });

  it('accepts valid IPv4 addresses only', () => {
    expect(validateDhcpStaticIpv4Address('192.168.10.2')).toBeNull();
    expect(validateDhcpStaticIpv4Address('300.168.10.2')).toBe('invalid_ip');
  });

  it('accepts queue rate limit only when both directions are present', () => {
    expect(validateDhcpStaticQueueRateLimit('20M/10M')).toBeNull();
    expect(validateDhcpStaticQueueRateLimit('20M')).toBe('invalid_queue_rate');
  });
});

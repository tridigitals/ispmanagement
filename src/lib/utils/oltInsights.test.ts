import { describe, expect, it } from 'vitest';
import {
  OLT_DRIVER_TYPES,
  friendlyOltError,
  formatBytes,
  formatUptime,
  hasOltDriver,
  onuStatusTone,
  oltTypeLabel,
  parseDbm,
  signalBand,
  signalColor,
  signalLabel,
  validateOltDraft,
} from './oltInsights';

const base = {
  name: 'OLT A',
  host: '10.0.0.1',
  port: 8021,
  username: 'admin',
  password: 'rahasia',
  oltType: 'hioso_ha7302cst',
  latitude: null,
  longitude: null,
  isNew: true,
};

describe('oltTypeLabel / drivers', () => {
  it('maps known types', () => {
    expect(oltTypeLabel('hioso_ha7302cst')).toContain('HIOSO');
    expect(oltTypeLabel('vsol_epon')).toContain('VSOL');
  });
  it('falls back to raw value for unknown', () => {
    expect(oltTypeLabel('future_box')).toBe('future_box');
    expect(oltTypeLabel(null)).toBe('—');
  });
  it('only driver-backed types pass hasOltDriver', () => {
    expect(hasOltDriver('hioso_ha7302cst')).toBe(true);
    expect(hasOltDriver('mikrotik_ros')).toBe(true);
    // vsol_epon ada di UI lama tapi create_driver() menolaknya -> bug nyata
    expect(hasOltDriver('vsol_epon')).toBe(false);
    expect(OLT_DRIVER_TYPES).not.toContain('vsol_epon');
  });
});

describe('signal bands', () => {
  it.each([
    [-15, 'good'],
    [-20.5, 'fair'],
    [-24, 'fair'],
    [-25, 'weak'],
    [-27, 'weak'],
    [-30, 'very_weak'],
  ] as const)('%i dBm -> %s', (dbm, band) => {
    expect(signalBand(dbm)).toBe(band);
  });
  it('null/NaN -> unknown', () => {
    expect(signalBand(null)).toBe('unknown');
    expect(signalBand(Number.NaN)).toBe('unknown');
    expect(signalLabel(null)).toBe('—');
    expect(signalColor(null)).toContain('--ds-ink-400');
  });
  it('labels are Indonesian', () => {
    expect(signalLabel(-15)).toBe('Baik');
    expect(signalLabel(-30)).toBe('Sangat lemah');
  });
});

describe('parseDbm', () => {
  it('parses strings incl. unicode minus and unit suffix', () => {
    expect(parseDbm('-21.5 dBm')).toBeCloseTo(-21.5);
    expect(parseDbm('−21.5')).toBeCloseTo(-21.5);
    expect(parseDbm(-22)).toBe(-22);
    expect(parseDbm('n/a')).toBeNull();
    expect(parseDbm(undefined)).toBeNull();
  });
});

describe('onuStatusTone', () => {
  it('maps statuses', () => {
    expect(onuStatusTone('online')).toBe('positive');
    expect(onuStatusTone('Offline')).toBe('negative');
    expect(onuStatusTone('LOS')).toBe('warning');
    expect(onuStatusTone('Dying Gasp')).toBe('warning');
    expect(onuStatusTone('whatever')).toBe('neutral');
    expect(onuStatusTone(null)).toBe('neutral');
  });
});

describe('formatUptime / formatBytes', () => {
  it('formats', () => {
    expect(formatUptime(90)).toBe('1m');
    expect(formatUptime(3725)).toBe('1j 2m');
    expect(formatUptime(90061)).toBe('1h 1j 1m');
    expect(formatUptime(null)).toBe('—');
    expect(formatUptime(-5)).toBe('—');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024 * 1024)).toBe('5.00 GB');
    expect(formatBytes(null)).toBe('—');
  });
});

describe('validateOltDraft', () => {
  it('accepts a complete new draft', () => {
    expect(validateOltDraft(base)).toEqual({});
  });
  it('requires password only for new', () => {
    expect(validateOltDraft({ ...base, password: ' ' }).password).toBeTruthy();
    expect(validateOltDraft({ ...base, password: '', isNew: false }).password).toBeUndefined();
  });
  it('rejects bad host/port', () => {
    expect(validateOltDraft({ ...base, host: 'bad host!' }).host).toBeTruthy();
    expect(validateOltDraft({ ...base, port: 0 }).port).toBeTruthy();
    expect(validateOltDraft({ ...base, port: 70000 }).port).toBeTruthy();
    expect(validateOltDraft({ ...base, port: null }).port).toBeTruthy();
  });
  it('lat/lng must pair and be in range', () => {
    expect(validateOltDraft({ ...base, latitude: -7 }).location).toMatch(/berpasangan/);
    expect(validateOltDraft({ ...base, latitude: -91, longitude: 110 }).location).toMatch(/Latitude/);
    expect(validateOltDraft({ ...base, latitude: -7, longitude: 181 }).location).toMatch(/Longitude/);
    expect(validateOltDraft({ ...base, latitude: -7.2, longitude: 110.4 })).toEqual({});
  });
});

describe('friendlyOltError', () => {
  it('translates FK + unsupported type', () => {
    expect(friendlyOltError('Unsupported OLT type: vsol_epon. Supported: ...')).toContain('driver');
    expect(friendlyOltError('violates foreign key constraint "fk_olts_uplink_router"')).toContain(
      'Router uplink',
    );
    expect(friendlyOltError('Connection failed: timeout')).toContain('timeout');
  });
  it('passes unknown through', () => {
    expect(friendlyOltError('weird')).toBe('weird');
  });
});

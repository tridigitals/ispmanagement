import { describe, expect, it } from 'vitest';
import {
  alertSeverityLabel,
  alertSeverityTone,
  alertSeverityWeight,
  alertStatusTone,
  alertTypeLabel,
  filterAlertRows,
} from './networkAlertInsights';

const rows = [
  { status: 'open', severity: 'critical', alert_type: 'offline', last_seen_at: '2026-09-05T10:00:00Z' },
  { status: 'ack', severity: 'warning', alert_type: 'cpu', last_seen_at: '2026-09-04T10:00:00Z' },
  { status: 'resolved', severity: 'info', alert_type: 'latency', last_seen_at: '2026-09-03T10:00:00Z' },
];

const base = { status: 'all', severity: 'all', type: 'all', from: '', to: '', sort: 'last_seen_desc' };

describe('label + tone', () => {
  it('bobot severity critical>warning>info>lain', () => {
    expect(alertSeverityWeight('critical')).toBe(3);
    expect(alertSeverityWeight('warning')).toBe(2);
    expect(alertSeverityWeight('info')).toBe(1);
    expect(alertSeverityWeight('x')).toBe(0);
  });
  it('label tipe + severity + fallback', () => {
    expect(alertTypeLabel('offline')).toBe('Offline');
    expect(alertTypeLabel('cpu')).toBe('CPU');
    expect(alertTypeLabel('latency')).toBe('Latensi');
    expect(alertTypeLabel('disk')).toBe('disk');
    expect(alertSeverityLabel('critical')).toBe('Kritis');
    expect(alertSeverityTone('critical')).toBe('negative');
    expect(alertStatusTone('resolved')).toBe('positive');
    expect(alertStatusTone('ack')).toBe('info');
  });
});

describe('filterAlertRows', () => {
  it('default urut last_seen_desc', () => {
    const out = filterAlertRows(rows, base);
    expect(out.map((r) => r.severity)).toEqual(['critical', 'warning', 'info']);
  });
  it('filter status + severity sort', () => {
    expect(filterAlertRows(rows, { ...base, status: 'open' }).length).toBe(1);
    const sev = filterAlertRows(rows, { ...base, sort: 'severity_desc' });
    expect(sev[0].severity).toBe('critical');
  });
  it('rentang tanggal from/to', () => {
    expect(filterAlertRows(rows, { ...base, from: '2026-09-04', to: '2026-09-04' }).length).toBe(1);
  });
});

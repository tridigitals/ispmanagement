import { describe, expect, it } from 'vitest';
import {
  nocFormatBps,
  nocHealthScore,
  nocIsHot,
  nocMatchesRisk,
  nocMemoryPct,
  type NocThresholds,
} from './nocInsights';

const TH: NocThresholds = { cpuRisk: 70, cpuHot: 85, latRisk: 200, latHot: 400 };

describe('nocHealthScore', () => {
  it('maintenance -1, offline 1000, sehat 0', () => {
    expect(nocHealthScore({ id: 'a', name: 'x', is_online: true, maintenance_until: new Date(Date.now() + 3600e3).toISOString() }, TH)).toBe(-1);
    expect(nocHealthScore({ id: 'a', name: 'x', is_online: false }, TH)).toBe(1000);
    expect(nocHealthScore({ id: 'a', name: 'x', is_online: true, cpu_load: 50, latency_ms: 100 }, TH)).toBe(0);
    expect(nocHealthScore({ id: 'a', name: 'x', is_online: true, cpu_load: 80, latency_ms: 250 }, TH)).toBe(10 + 5);
  });
});

describe('nocIsHot + nocMatchesRisk', () => {
  it('offline panas, maintenance diredam', () => {
    const off = { id: 'a', name: 'x', is_online: false };
    expect(nocIsHot(off, TH)).toBe(true);
    const maint = { id: 'b', name: 'y', is_online: false, maintenance_until: new Date(Date.now() + 3600e3).toISOString() };
    expect(nocIsHot(maint, TH)).toBe(false);
    expect(nocMatchesRisk(maint, 'hot', TH)).toBe(false);
  });
  it('ambang cpu/latency', () => {
    const cpu = { id: 'c', name: 'z', is_online: true, cpu_load: 75, latency_ms: 10 };
    expect(nocMatchesRisk(cpu, 'cpu', TH)).toBe(true);
    expect(nocMatchesRisk(cpu, 'latency', TH)).toBe(false);
  });
});

describe('nocMemoryPct + nocFormatBps', () => {
  it('persen terpakai + format throughput', () => {
    expect(nocMemoryPct(100, 30)).toBe(70);
    expect(nocMemoryPct(0, 0)).toBe(null);
    expect(nocFormatBps(null)).toBe('—');
    expect(nocFormatBps(500)).toBe('500 bps');
    expect(nocFormatBps(1500)).toBe('1.5 Kbps');
    expect(nocFormatBps(25_000_000)).toBe('25 Mbps');
  });
});

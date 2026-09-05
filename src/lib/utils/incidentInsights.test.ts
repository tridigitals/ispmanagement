import { describe, expect, it } from 'vitest';
import {
  formatDurationCompact,
  friendlyIncidentError,
  incidentCounts,
  incidentOpenMs,
  incidentStartMs,
  meanTimeToAck,
  meanTimeToResolve,
  severityLabel,
  severityWeight,
  slaLevel,
  statusLabel,
  type IncidentLike,
} from './incidentInsights';

const NOW = Date.parse('2026-09-05T12:00:00Z');

function row(over: Partial<IncidentLike> = {}): IncidentLike {
  return {
    status: 'open',
    severity: 'warning',
    updated_at: '2026-09-05T10:00:00Z',
    ...over,
  };
}

describe('severity / status label', () => {
  it('bobot urut kritis > warning > info > lain', () => {
    expect(severityWeight('critical')).toBe(3);
    expect(severityWeight('warning')).toBe(2);
    expect(severityWeight('info')).toBe(1);
    expect(severityWeight('unknown')).toBe(0);
  });
  it('label Indonesia', () => {
    expect(severityLabel('critical')).toBe('Kritis');
    expect(statusLabel('in_progress')).toBe('Ditangani');
    expect(statusLabel('weird')).toBe('weird');
  });
});

describe('incidentStartMs / openMs', () => {
  it('first_seen_at menang; fallback updated_at lalu last_seen_at', () => {
    expect(incidentStartMs(row({ first_seen_at: '2026-09-01T00:00:00Z' }))).toBe(
      Date.parse('2026-09-01T00:00:00Z'),
    );
    expect(incidentStartMs(row({ updated_at: '2026-09-02T00:00:00Z' }))).toBe(
      Date.parse('2026-09-02T00:00:00Z'),
    );
    const noUpdated = { ...row(), updated_at: '', last_seen_at: '2026-09-03T00:00:00Z' };
    expect(incidentStartMs(noUpdated)).toBe(Date.parse('2026-09-03T00:00:00Z'));
  });
  it('resolved -> berhenti di resolved_at; aktif -> sampai now', () => {
    const r = row({ resolved_at: '2026-09-05T11:00:00Z' });
    expect(incidentOpenMs(r, NOW)).toBe(3600000);
    expect(incidentOpenMs(row(), NOW)).toBe(2 * 3600000);
  });
  it('tanggal rusak / end < start -> 0', () => {
    expect(incidentOpenMs(row({ resolved_at: 'bukan-tanggal' }), NOW)).toBe(0);
    expect(
      incidentOpenMs(row({ first_seen_at: '2026-09-06T00:00:00Z', resolved_at: '2026-09-05T00:00:00Z' }), NOW),
    ).toBe(0);
  });
});

describe('formatDurationCompact', () => {
  it('satuan bertingkat', () => {
    expect(formatDurationCompact(30000)).toBe('<1m');
    expect(formatDurationCompact(45 * 60000)).toBe('45m');
    expect(formatDurationCompact((3 * 3600 + 12 * 60) * 1000)).toBe('3j 12m');
    expect(formatDurationCompact((40 * 86400 + 3 * 3600) * 1000)).toBe('40d 3j');
    expect(formatDurationCompact(-5)).toBe('—');
  });
});

describe('MTTA / MTTR', () => {
  it('hanya sampel yang punya ack/resolved', () => {
    const rows = [
      row({ acked_at: '2026-09-05T10:30:00Z' }), // 30 menit
      row({ acked_at: '2026-09-05T11:00:00Z' }), // 60 menit
      row(), // tanpa ack
    ];
    expect(meanTimeToAck(rows, NOW)).toBe(45);
  });
  it('tanpa sampel -> null', () => {
    expect(meanTimeToAck([row()], NOW)).toBeNull();
    expect(meanTimeToResolve([row()])).toBeNull();
  });
  it('MTTR dari resolved_at', () => {
    const rows = [
      row({ resolved_at: '2026-09-05T10:20:00Z' }), // 20 menit
      row({ resolved_at: '2026-09-05T10:40:00Z' }), // 40 menit
    ];
    expect(meanTimeToResolve(rows)).toBe(30);
  });
});

describe('slaLevel', () => {
  it('resolved selalu ok', () => {
    expect(slaLevel(row({ status: 'resolved', first_seen_at: '2020-01-01T00:00:00Z' }), NOW, 1, 2)).toBe('ok');
  });
  it('ambang warn/breach dari input (menit)', () => {
    const open2h = row({ first_seen_at: '2026-09-05T10:00:00Z' });
    expect(slaLevel(open2h, NOW, 120, 480)).toBe('warn'); // 120 >= 120
    expect(slaLevel(open2h, NOW, 30, 120)).toBe('breach'); // 120 >= 120
    expect(slaLevel(open2h, NOW, 240, 480)).toBe('ok');
  });
});

describe('incidentCounts', () => {
  it('memecah per status, status asing tidak dihitung', () => {
    const rows = [
      row({ status: 'open' }),
      row({ status: 'ack' }),
      row({ status: 'in_progress' }),
      row({ status: 'resolved' }),
      row({ status: 'resolved' }),
      row({ status: 'bogus' }),
    ];
    expect(incidentCounts(rows)).toEqual({ open: 1, ack: 1, inProgress: 1, resolved: 2 });
  });
});

describe('friendlyIncidentError', () => {
  it('peta pesan guard baru', () => {
    expect(friendlyIncidentError('Incident not found')).toContain('tidak ditemukan');
    expect(friendlyIncidentError('Incident already resolved')).toContain('sudah selesai');
    expect(friendlyIncidentError('owner_user_id is not a member of this tenant')).toContain(
      'anggota tenant',
    );
  });
  it('kosong -> pesan umum', () => {
    expect(friendlyIncidentError('')).toContain('kesalahan');
  });
});

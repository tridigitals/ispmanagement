import { describe, expect, it } from 'vitest';
import {
  buildChecklistNote,
  buildPersistedNotes,
  friendlyWorkOrderError,
  parseChecklistState,
  parsePhotoIds,
  stripGeneratedSections,
  woStatusLabel,
} from './installationNotes';

describe('parseChecklistState', () => {
  it('membaca checklist generated dengan benar', () => {
    const notes =
      'Catatan dispatcher\n\nInstallation checklist:\n[x] Cable installed\n[ ] ONT installed\n[x] PPPoE configured\n[ ] Speed test passed';
    expect(parseChecklistState(notes)).toEqual({
      cable: true,
      ont: false,
      pppoe: true,
      speed: false,
    });
  });

  it('case-insensitive dan toleran spasi ganda', () => {
    expect(parseChecklistState('[X]   ONT installed').ont).toBe(true);
  });

  it('notes kosong -> semua false', () => {
    expect(parseChecklistState(null)).toEqual({
      cable: false,
      ont: false,
      pppoe: false,
      speed: false,
    });
  });
});

describe('stripGeneratedSections', () => {
  it('menghapus blok checklist dan foto, menyisakan teks manual', () => {
    const notes = [
      'Pelanggan minta pagi.',
      '',
      'Installation checklist:',
      '[x] Cable installed',
      '[ ] ONT installed',
      '[ ] PPPoE configured',
      '[ ] Speed test passed',
      '',
      'Installation photos:',
      '- ont.jpg: https://api.example/storage/files/abc123def/content',
    ].join('\n');
    expect(stripGeneratedSections(notes)).toBe('Pelanggan minta pagi.');
  });

  it('idempoten: hasil strip tidak mengubah notes bersih', () => {
    const clean = 'hanya catatan manual';
    expect(stripGeneratedSections(clean)).toBe(clean);
  });
});

describe('buildChecklistNote + buildPersistedNotes', () => {
  it('round-trip: state -> note -> state', () => {
    const state = { cable: true, ont: false, pppoe: true, speed: true };
    expect(parseChecklistState(buildChecklistNote(state))).toEqual(state);
  });

  it('persisted notes: manual + checklist + foto, bagian kosong dilewati', () => {
    const out = buildPersistedNotes(
      'Catatan lama',
      { cable: true, ont: false, pppoe: false, speed: false },
      ['- a.jpg: https://x/storage/files/f1/content'],
    );
    expect(out).toContain('Catatan lama');
    expect(out).toContain('[x] Cable installed');
    expect(out).toContain('Installation photos:');
    // notes lama yang sudah mengandung blok generated tidak dobel
    expect(stripGeneratedSections(out)).toBe('Catatan lama');
  });

  it('tanpa manual dan tanpa foto -> hanya checklist', () => {
    const out = buildPersistedNotes(
      '',
      { cable: false, ont: false, pppoe: false, speed: false },
      [],
    );
    expect(out.startsWith('Installation checklist:')).toBe(true);
    expect(out).not.toContain('Installation photos');
  });
});

describe('parsePhotoIds', () => {
  it('ekstrak id unik dari URL content', () => {
    const notes =
      'a https://api/storage/files/11111111-2222-3333-4444-555566667777/content b ' +
      'https://api/storage/files/11111111-2222-3333-4444-555566667777/content';
    expect(parsePhotoIds(notes)).toEqual(['11111111-2222-3333-4444-555566667777']);
  });

  it('URL tanpa /content diabaikan (sama seperti legacy)', () => {
    expect(parsePhotoIds('/storage/files/abcdef01/download')).toEqual([]);
  });
});

describe('woStatusLabel + friendlyWorkOrderError', () => {
  it('label status', () => {
    expect(woStatusLabel('in_progress')).toBe('Dikerjakan');
    expect(woStatusLabel('weird')).toBe('weird');
  });

  it('race claim jadi kalimat jelas', () => {
    expect(friendlyWorkOrderError('Work order already taken by another technician')).toContain(
      'diambil teknisi lain',
    );
  });

  it('guard admin/owner reopen/cancel', () => {
    expect(friendlyWorkOrderError('Only admin/owner can reopen installation work orders')).toContain(
      'admin/owner',
    );
  });

  it('error tak dikenal diteruskan apa adanya', () => {
    expect(friendlyWorkOrderError('DB timeout')).toBe('DB timeout');
    expect(friendlyWorkOrderError('')).toBe('Terjadi kesalahan tak terduga.');
  });
});

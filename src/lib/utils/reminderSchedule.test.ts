import { describe, expect, it } from 'vitest';

import {
  REMINDER_PRESET_DETAILS,
  REMINDER_PRESETS,
  addReminderCode,
  buildReminderCode,
  formatReminderCodeLabel,
  groupReminderCodes,
  parseReminderSchedule,
  removeReminderCode,
  stringifyReminderSchedule,
} from './reminderSchedule';

describe('reminder schedule helpers', () => {
  it('parses valid reminder codes, deduplicates, and sorts them', () => {
    expect(parseReminderSchedule('H+3, h-1, foo, H+3, H-7')).toEqual(['H-7', 'H-1', 'H+3']);
  });

  it('builds reminder codes with clamped days', () => {
    expect(buildReminderCode('before', 0)).toBe('H-1');
    expect(buildReminderCode('after', 40)).toBe('H+30');
  });

  it('adds and removes reminder codes safely', () => {
    const added = addReminderCode(['H-3', 'H+1'], 'H-1');
    expect(added).toEqual(['H-3', 'H-1', 'H+1']);
    expect(removeReminderCode(added, 'H-3')).toEqual(['H-1', 'H+1']);
  });

  it('groups reminder codes into before and after due date buckets', () => {
    expect(groupReminderCodes(['H-3', 'H+1', 'H-1', 'H+3'])).toEqual({
      before: ['H-3', 'H-1'],
      after: ['H+1', 'H+3'],
    });
  });

  it('stringifies presets into stable schedule strings', () => {
    expect(stringifyReminderSchedule([...REMINDER_PRESETS.standard])).toBe('H-3,H-1,H+1,H+3');
  });

  it('keeps preset labels and descriptions available for the UI', () => {
    expect(REMINDER_PRESET_DETAILS.light.label).toBe('Ringan');
    expect(REMINDER_PRESET_DETAILS.standard.description).toContain('mayoritas pelanggan');
    expect(REMINDER_PRESET_DETAILS.aggressive.description).toContain('terlambatan');
  });

  it('formats reminder codes into human-readable labels', () => {
    expect(formatReminderCodeLabel('H-3')).toBe('3 hari sebelum');
    expect(formatReminderCodeLabel('H+1')).toBe('1 hari sesudah');
  });
});

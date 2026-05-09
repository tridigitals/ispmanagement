export type ReminderTiming = 'before' | 'after';

export const REMINDER_PRESETS = {
  standard: ['H-3', 'H-1', 'H+1', 'H+3'],
  light: ['H-1', 'H+1'],
  aggressive: ['H-7', 'H-3', 'H-1', 'H+1', 'H+3', 'H+7'],
} as const;

export const REMINDER_PRESET_DETAILS = {
  light: {
    label: 'Ringan',
    description: 'Cocok jika Anda ingin reminder seperlunya tanpa terlalu sering follow-up.',
  },
  standard: {
    label: 'Standar',
    description: 'Pilihan aman untuk mayoritas pelanggan dengan ritme penagihan normal.',
  },
  aggressive: {
    label: 'Agresif',
    description: 'Lebih intens untuk menekan keterlambatan pembayaran sejak awal.',
  },
} as const;

export function parseReminderSchedule(raw: string | null | undefined): string[] {
  const parsed: string[] = [];
  for (const token of String(raw || '').split(',')) {
    const item = token.trim().toUpperCase();
    if (!/^H[+-]\d+$/.test(item) || parsed.includes(item)) continue;
    parsed.push(item);
  }
  return parsed.sort(compareReminderCode);
}

export function stringifyReminderSchedule(codes: string[]): string {
  return [...codes].sort(compareReminderCode).join(',');
}

export function compareReminderCode(a: string, b: string): number {
  return reminderCodeValue(a) - reminderCodeValue(b);
}

export function reminderCodeValue(code: string): number {
  const match = /^H([+-])(\d+)$/.exec(code.trim().toUpperCase());
  if (!match) return 0;
  const sign = match[1] === '-' ? -1 : 1;
  return sign * Number(match[2]);
}

export function buildReminderCode(timing: ReminderTiming, days: number): string {
  const clamped = Math.max(1, Math.min(30, Math.trunc(days)));
  return `H${timing === 'before' ? '-' : '+'}${clamped}`;
}

export function addReminderCode(codes: string[], code: string): string[] {
  const normalized = code.trim().toUpperCase();
  if (!/^H[+-]\d+$/.test(normalized)) return [...codes].sort(compareReminderCode);
  return Array.from(new Set([...codes, normalized])).sort(compareReminderCode);
}

export function removeReminderCode(codes: string[], code: string): string[] {
  const normalized = code.trim().toUpperCase();
  return codes.filter((item) => item !== normalized).sort(compareReminderCode);
}

export function groupReminderCodes(codes: string[]): {
  before: string[];
  after: string[];
} {
  const before: string[] = [];
  const after: string[] = [];
  for (const code of [...codes].sort(compareReminderCode)) {
    if (reminderCodeValue(code) < 0) before.push(code);
    else after.push(code);
  }
  return { before, after };
}

export function formatReminderCodeLabel(code: string): string {
  const value = reminderCodeValue(code);
  const days = Math.abs(value);
  if (!days) return code.trim().toUpperCase();
  return value < 0 ? `${days} hari sebelum` : `${days} hari sesudah`;
}

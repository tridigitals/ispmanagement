export type GlobalAutoSuspendMode = 'grace_period' | 'fixed_day';

export type GlobalAutoSuspendPolicy = {
  enabled: boolean;
  mode: GlobalAutoSuspendMode;
  graceDays: number;
  fixedDay: number;
};

export type CustomerSubscriptionPolicySummary = {
  activeUntilIso: string | null;
  activeUntilMissing: boolean;
  policyLabel: string;
  estimatedSuspendIso: string | null;
  estimatedSuspendMissingReason: string | null;
};

export function clampFixedSuspendDay(day: number): number {
  if (!Number.isFinite(day)) return 1;
  return Math.max(1, Math.min(28, Math.trunc(day)));
}

function normalizeMode(mode: string | null | undefined): GlobalAutoSuspendMode {
  return mode === 'fixed_day' ? 'fixed_day' : 'grace_period';
}

function parseDateOnly(value: string): { year: number; month: number; day: number } | null {
  const dateOnly = value.trim().slice(0, 10);
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dateOnly);
  if (!match) return null;

  return {
    year: Number(match[1]),
    month: Number(match[2]),
    day: Number(match[3]),
  };
}

function addDaysIso(value: string, days: number): string | null {
  const parsed = parseDateOnly(value);
  if (!parsed) return null;

  const utc = new Date(Date.UTC(parsed.year, parsed.month - 1, parsed.day));
  utc.setUTCDate(utc.getUTCDate() + Math.max(0, Math.trunc(days)));
  return utc.toISOString().slice(0, 10);
}

function nextFixedDayIso(value: string, fixedDay: number): string | null {
  const parsed = parseDateOnly(value);
  if (!parsed) return null;

  const targetDay = clampFixedSuspendDay(fixedDay);
  if (parsed.day <= targetDay) {
    return `${String(parsed.year).padStart(4, '0')}-${String(parsed.month).padStart(2, '0')}-${String(targetDay).padStart(2, '0')}`;
  }

  const monthIndex = parsed.month;
  const nextYear = monthIndex === 12 ? parsed.year + 1 : parsed.year;
  const nextMonth = monthIndex === 12 ? 1 : monthIndex + 1;
  return `${String(nextYear).padStart(4, '0')}-${String(nextMonth).padStart(2, '0')}-${String(targetDay).padStart(2, '0')}`;
}

export function buildCustomerSubscriptionPolicySummary(args: {
  endsAt: string | null;
  policy: GlobalAutoSuspendPolicy;
}): CustomerSubscriptionPolicySummary {
  const mode = normalizeMode(args.policy.mode);
  const graceDays = Math.max(0, Math.trunc(args.policy.graceDays));
  const fixedDay = clampFixedSuspendDay(args.policy.fixedDay);
  const policyLabel =
    mode === 'fixed_day'
      ? `Suspend tanggal ${fixedDay} setiap bulan`
      : `Grace ${graceDays} hari setelah masa aktif`;

  const activeUntilIso = args.endsAt ? args.endsAt.slice(0, 10) : null;

  if (!activeUntilIso) {
    return {
      activeUntilIso: null,
      activeUntilMissing: true,
      policyLabel,
      estimatedSuspendIso: null,
      estimatedSuspendMissingReason: 'Suspend otomatis tidak bisa dihitung',
    };
  }

  const estimatedSuspendIso =
    mode === 'fixed_day'
      ? nextFixedDayIso(activeUntilIso, fixedDay)
      : addDaysIso(activeUntilIso, graceDays);

  return {
    activeUntilIso,
    activeUntilMissing: false,
    policyLabel,
    estimatedSuspendIso,
    estimatedSuspendMissingReason: estimatedSuspendIso ? null : 'Suspend otomatis tidak bisa dihitung',
  };
}

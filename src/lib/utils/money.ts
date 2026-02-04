import { get } from 'svelte/store';
import { appSettings } from '$lib/stores/settings';

export type MoneyFormatOptions = {
  currency?: string;
  locale?: string;
  maximumFractionDigits?: number;
  minimumFractionDigits?: number;
};

export function formatMoney(amount: number, options: MoneyFormatOptions = {}): string {
  const settings = get(appSettings);
  const currency = options.currency || (settings as any).currency_code || 'IDR';
  const locale = options.locale || settings.default_locale || 'en-US';

  const safeAmount = Number.isFinite(amount) ? amount : 0;

  try {
    return new Intl.NumberFormat(locale, {
      style: 'currency',
      currency: currency,
      maximumFractionDigits: options.maximumFractionDigits,
      minimumFractionDigits: options.minimumFractionDigits,
    }).format(safeAmount);
  } catch {
    // Fallback for invalid currency codes / environments
    return `${safeAmount.toFixed(2)} ${currency}`;
  }
}

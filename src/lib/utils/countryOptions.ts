/**
 * Canonical list of ISO 3166-1 alpha-2 country codes we ship in our
 * address pickers. Indonesia is intentionally listed first and is
 * the default — the business primarily serves Indonesian ISPs.
 *
 * The list is a shallow copy of the existing
 * `routes/(app)/admin/customers/orders/new/countryOptions.ts`. We
 * lifted it into `$lib/utils/` so any feature page (registration,
 * profile, customer self-service, etc.) can reuse the same list
 * without re-declaring the data on every page.
 */
export type CountryOption = {
  value: string;
  label: string;
};

const COUNTRIES: CountryOption[] = [
  { value: 'ID', label: 'Indonesia (ID)' },
  { value: 'SG', label: 'Singapore (SG)' },
  { value: 'MY', label: 'Malaysia (MY)' },
  { value: 'TH', label: 'Thailand (TH)' },
  { value: 'VN', label: 'Vietnam (VN)' },
  { value: 'PH', label: 'Philippines (PH)' },
  { value: 'BN', label: 'Brunei Darussalam (BN)' },
  { value: 'KH', label: 'Cambodia (KH)' },
  { value: 'LA', label: 'Laos (LA)' },
  { value: 'MM', label: 'Myanmar (MM)' },
  { value: 'TL', label: 'Timor-Leste (TL)' },
  { value: 'CN', label: 'China (CN)' },
  { value: 'HK', label: 'Hong Kong (HK)' },
  { value: 'TW', label: 'Taiwan (TW)' },
  { value: 'JP', label: 'Japan (JP)' },
  { value: 'KR', label: 'South Korea (KR)' },
  { value: 'IN', label: 'India (IN)' },
  { value: 'AE', label: 'United Arab Emirates (AE)' },
  { value: 'SA', label: 'Saudi Arabia (SA)' },
  { value: 'AU', label: 'Australia (AU)' },
  { value: 'NZ', label: 'New Zealand (NZ)' },
  { value: 'GB', label: 'United Kingdom (GB)' },
  { value: 'DE', label: 'Germany (DE)' },
  { value: 'FR', label: 'France (FR)' },
  { value: 'NL', label: 'Netherlands (NL)' },
  { value: 'US', label: 'United States (US)' },
  { value: 'CA', label: 'Canada (CA)' },
  { value: 'BR', label: 'Brazil (BR)' },
  { value: 'ZA', label: 'South Africa (ZA)' },
];

export function buildCountryOptions(): CountryOption[] {
  return COUNTRIES;
}

type PhonePrefixOption = {
  value: string;
  label: string;
};

const PHONE_PREFIX_OPTIONS: PhonePrefixOption[] = [
  { value: '+62', label: 'Indonesia (+62)' },
  { value: '+65', label: 'Singapore (+65)' },
  { value: '+60', label: 'Malaysia (+60)' },
  { value: '+66', label: 'Thailand (+66)' },
  { value: '+84', label: 'Vietnam (+84)' },
  { value: '+63', label: 'Philippines (+63)' },
  { value: '+673', label: 'Brunei (+673)' },
  { value: '+855', label: 'Cambodia (+855)' },
  { value: '+856', label: 'Laos (+856)' },
  { value: '+95', label: 'Myanmar (+95)' },
  { value: '+670', label: 'Timor-Leste (+670)' },
  { value: '+86', label: 'China (+86)' },
  { value: '+81', label: 'Japan (+81)' },
  { value: '+82', label: 'South Korea (+82)' },
  { value: '+91', label: 'India (+91)' },
  { value: '+971', label: 'United Arab Emirates (+971)' },
  { value: '+61', label: 'Australia (+61)' },
  { value: '+64', label: 'New Zealand (+64)' },
  { value: '+44', label: 'United Kingdom (+44)' },
  { value: '+49', label: 'Germany (+49)' },
  { value: '+33', label: 'France (+33)' },
  { value: '+31', label: 'Netherlands (+31)' },
  { value: '+1', label: 'United States (+1)' },
  { value: '+1-CA', label: 'Canada (+1)' },
];

function sanitizeLocalNumber(value: string): string {
  return value.replace(/[^\d]/g, '');
}

export function composePhoneNumber(prefix: string, localNumber: string): string {
  const normalizedPrefix = prefix.trim();
  const normalizedLocalNumber = sanitizeLocalNumber(localNumber);
  if (!normalizedPrefix && !normalizedLocalNumber) return '';
  if (!normalizedPrefix) return normalizedLocalNumber;
  return `${normalizedPrefix.replace(/-CA$/, '')}${normalizedLocalNumber}`;
}

export function inferPhoneFieldState(value: string): { prefix: string; localNumber: string } {
  const phone = value.trim();
  const matchingPrefix = PHONE_PREFIX_OPTIONS
    .map((option) => option.value.replace(/-CA$/, ''))
    .sort((a, b) => b.length - a.length)
    .find((prefix) => phone.startsWith(prefix));

  if (matchingPrefix) {
    return {
      prefix: matchingPrefix,
      localNumber: sanitizeLocalNumber(phone.slice(matchingPrefix.length)),
    };
  }

  return {
    prefix: '+62',
    localNumber: sanitizeLocalNumber(phone),
  };
}

export function buildPhonePrefixOptions(): PhonePrefixOption[] {
  return PHONE_PREFIX_OPTIONS.map((option) => ({
    value: option.value.replace(/-CA$/, ''),
    label: option.label,
  }));
}

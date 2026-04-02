type SecretShape = {
  shared_secret?: string | null;
  shared_secret_masked?: string | null;
};

export function getManagedRadiusDisplayedSecret(
  setup: SecretShape | null | undefined,
  revealed: boolean,
): string {
  if (!setup) return '—';
  if (revealed && setup.shared_secret) return setup.shared_secret;
  if (setup.shared_secret_masked) return setup.shared_secret_masked;
  if (setup.shared_secret) return setup.shared_secret;
  return '—';
}

export function canCopyManagedRadiusSecret(setup: SecretShape | null | undefined): boolean {
  return Boolean(setup?.shared_secret);
}

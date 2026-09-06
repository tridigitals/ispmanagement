/**
 * Helper murni daftar next-pool IP v2 (gelombang 24d).
 *
 * Opsi next-pool (unik, sortir, kecualikan pool yg sedang diedit) dulu
 * inline di halaman legacy — kini murni + tes.
 */
export function ipPoolNextPoolOptions(names: (string | null | undefined)[], currentName: string): string[] {
  return names
    .map((n) => n?.trim())
    .filter((n): n is string => Boolean(n))
    .filter((n, i, all) => all.indexOf(n) === i)
    .filter((n) => n !== currentName.trim())
    .sort((a, b) => a.localeCompare(b));
}

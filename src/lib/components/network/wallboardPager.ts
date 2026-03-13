export function normalizeSlotsForAutoSpare<T>(
  input: (T | null)[],
  pageSize: number,
): (T | null)[] {
  if (pageSize <= 0) return input;

  let lastUsed = -1;
  for (let i = input.length - 1; i >= 0; i--) {
    if (input[i]) {
      lastUsed = i;
      break;
    }
  }

  const usedLen = Math.max(0, lastUsed + 1);
  const basePages = Math.max(1, Math.ceil(usedLen / pageSize));
  const baseLen = basePages * pageSize;
  const allFilled = usedLen > 0 && usedLen === baseLen;
  const targetLen = allFilled ? baseLen + pageSize : baseLen;

  const out = input.slice(0, targetLen);
  if (out.length < targetLen) {
    out.push(...Array.from({ length: targetLen - out.length }, () => null));
  }
  return out;
}

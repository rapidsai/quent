/**
 * TODO: Figure out a more permanent solution for this
 * Parse JSON with BigInt support for large integers.
 * Integers larger than Number.MAX_SAFE_INTEGER are converted to BigInt.
 */
export function parseJsonWithBigInt<T>(text: string): T {
  // Match integers that are too large for Number (and not floats)
  // This regex finds: a number boundary, optional minus, digits only (no decimal/exponent)
  // We convert integers > MAX_SAFE_INTEGER to BigInt
  const processed = text.replace(
    /([:\s[,]|^)(-?\d{16,})(?=[,\s}\]]|$)/g,
    (match, prefix, numStr) => {
      const num = Number(numStr);
      // Only convert if it exceeds safe integer range
      if (!Number.isSafeInteger(num)) {
        return `${prefix}"__bigint__${numStr}"`;
      }
      return match;
    }
  );

  return JSON.parse(processed, (_key, value) => {
    if (typeof value === 'string' && value.startsWith('__bigint__')) {
      return BigInt(value.slice(10));
    }
    return value;
  });
}

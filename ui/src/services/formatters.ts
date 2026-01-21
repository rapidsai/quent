/**
 * Centralized formatting utilities for charts and UI.
 */

/**
 * Format a number with abbreviated suffixes (K, M, B, T).
 * @param value - The number to format
 * @param decimals - Number of decimal places (default: 1)
 * @returns Formatted string (e.g., "1.5M", "320K", "42")
 */
export function formatAbbreviatedNumber(value: number, decimals: number = 1): string {
  const absValue = Math.abs(value);
  const sign = value < 0 ? '-' : '';

  if (absValue >= 1_000_000_000_000) {
    return `${sign}${(absValue / 1_000_000_000_000).toFixed(decimals)}T`;
  }
  if (absValue >= 1_000_000_000) {
    return `${sign}${(absValue / 1_000_000_000).toFixed(decimals)}B`;
  }
  if (absValue >= 1_000_000) {
    return `${sign}${(absValue / 1_000_000).toFixed(decimals)}M`;
  }
  if (absValue >= 1_000) {
    return `${sign}${(absValue / 1_000).toFixed(decimals)}K`;
  }
  return value.toString();
}

/**
 * Format a number with abbreviated suffixes, removing trailing zeros.
 * @param value - The number to format
 * @param maxDecimals - Maximum decimal places (default: 1)
 * @returns Formatted string (e.g., "1.5M", "2M", "320K")
 */
export function formatAbbreviatedNumberCompact(value: number, maxDecimals: number = 1): string {
  const formatted = formatAbbreviatedNumber(value, maxDecimals);
  // Remove trailing zeros after decimal point (e.g., "1.0M" -> "1M")
  return formatted.replace(/\.0+([KMBT]?)$/, '$1');
}

/**
 * Format epoch microseconds to a localized date string.
 * @param epochMicros - Timestamp in microseconds
 * @returns Formatted date string
 */
export function formatEpochMicros(epochMicros: number): string {
  return new Date(epochMicros / 1000).toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

const BYTE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'] as const;
const BYTE_UNITS_BINARY = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'] as const;

/**
 * Format bytes with appropriate unit suffix.
 * @param bytes - Number of bytes
 * @param decimals - Number of decimal places (default: 1)
 * @param binary - Use binary units (1024-based: KiB, MiB) vs decimal (1000-based: KB, MB). Default: true
 * @returns Formatted string (e.g., "1.5 GB", "320 KB", "42 B")
 */
export function formatBytes(bytes: number, decimals: number = 1, binary: boolean = true): string {
  if (bytes === 0) return '0 B';

  const absBytes = Math.abs(bytes);
  const sign = bytes < 0 ? '-' : '';
  const base = binary ? 1024 : 1000;
  const units = binary ? BYTE_UNITS_BINARY : BYTE_UNITS;

  const exponent = Math.min(Math.floor(Math.log(absBytes) / Math.log(base)), units.length - 1);
  const value = absBytes / Math.pow(base, exponent);
  const unit = units[exponent];

  return `${sign}${value.toFixed(decimals)} ${unit}`;
}

/**
 * Format bytes with appropriate unit suffix, removing trailing zeros.
 * @param bytes - Number of bytes
 * @param maxDecimals - Maximum decimal places (default: 1)
 * @param binary - Use binary units (1024-based) vs decimal (1000-based). Default: true
 * @returns Formatted string (e.g., "1.5 GB", "2 MB", "320 KB")
 */
export function formatBytesCompact(
  bytes: number,
  maxDecimals: number = 1,
  binary: boolean = true
): string {
  const formatted = formatBytes(bytes, maxDecimals, binary);
  // Remove trailing zeros after decimal point (e.g., "1.0 GB" -> "1 GB")
  return formatted.replace(/\.0+ /, ' ');
}

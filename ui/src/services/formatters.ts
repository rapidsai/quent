/**
 * Centralized formatting utilities for charts and UI.
 */

import type { PrefixSystem } from '~quent/types/PrefixSystem';
import type { QuantitySpec } from '~quent/types/QuantitySpec';
import type { CapacityKind } from '~quent/types/CapacityKind';

const MS_PER_SECOND = 1000;
const MS_PER_MINUTE = 60 * MS_PER_SECOND;
const MS_PER_HOUR = 60 * MS_PER_MINUTE;
const MS_PER_DAY = 24 * MS_PER_HOUR;

export function formatDuration(ms: number, decimals: number = 2): string {
  const absMs = Math.abs(ms);
  const sign = ms < 0 ? '-' : '';

  if (absMs < MS_PER_SECOND) {
    return `${sign}${absMs.toFixed(0)}ms`;
  }
  if (absMs < MS_PER_MINUTE) {
    return `${sign}${(absMs / MS_PER_SECOND).toFixed(decimals)}s`;
  }
  if (absMs < MS_PER_HOUR) {
    return `${sign}${(absMs / MS_PER_MINUTE).toFixed(decimals)}min`;
  }
  if (absMs < MS_PER_DAY) {
    return `${sign}${(absMs / MS_PER_HOUR).toFixed(decimals)}h`;
  }
  return `${sign}${(absMs / MS_PER_DAY).toFixed(decimals)}d`;
}

// Precomputed threshold/divisor tables to avoid Math.log/Math.pow per call.
const SI_UP: readonly [number, string][] = [
  [1e15, 'P'],
  [1e12, 'T'],
  [1e9, 'G'],
  [1e6, 'M'],
  [1e3, 'k'],
  [1, ''],
];
const SI_DOWN: readonly [number, string][] = [
  [1, ''],
  [1e-3, 'm'],
  [1e-6, 'µ'],
  [1e-9, 'n'],
  [1e-12, 'p'],
];
const IEC: readonly [number, string][] = [
  [1125899906842624, 'Pi'],
  [1099511627776, 'Ti'],
  [1073741824, 'Gi'],
  [1048576, 'Mi'],
  [1024, 'Ki'],
  [1, ''],
];

function formatWithPrefix(
  value: number,
  symbol: string,
  prefixSystem: PrefixSystem,
  decimals: number = 1
): string {
  if (value === 0) return symbol ? `0 ${symbol}` : '0';

  const abs = value < 0 ? -value : value;
  const sign = value < 0 ? '-' : '';

  if (prefixSystem === 'None') {
    return symbol ? `${sign}${abs.toFixed(decimals)} ${symbol}` : `${sign}${abs.toFixed(decimals)}`;
  }

  if (prefixSystem === 'Si' && abs < 1) {
    for (let i = 1; i < SI_DOWN.length; i++) {
      if (abs >= SI_DOWN[i][0]) {
        const scaled = abs / SI_DOWN[i][0];
        return `${sign}${scaled.toFixed(decimals)} ${SI_DOWN[i][1]}${symbol}`;
      }
    }
    const last = SI_DOWN[SI_DOWN.length - 1];
    return `${sign}${(abs / last[0]).toFixed(decimals)} ${last[1]}${symbol}`;
  }

  const table = prefixSystem === 'Iec' ? IEC : SI_UP;
  for (let i = 0; i < table.length; i++) {
    if (abs >= table[i][0]) {
      const scaled = abs / table[i][0];
      const prefix = table[i][1];
      return `${sign}${scaled.toFixed(decimals)} ${prefix}${symbol}`;
    }
  }
  const last = table[table.length - 1];
  return `${sign}${(abs / last[0]).toFixed(decimals)} ${last[1]}${symbol}`;
}

/**
 * Format a value using a QuantitySpec and CapacityKind.
 * Selects the appropriate prefix system based on the capacity kind.
 */
export function formatQuantity(
  value: number,
  spec: QuantitySpec,
  kind: CapacityKind,
  decimals: number = 2
): string {
  const prefixSystem = kind === 'Occupancy' ? spec.occupancy_prefix : spec.rate_prefix;
  const symbol = kind === 'Rate' ? `${spec.symbol}/s` : spec.symbol;
  return formatWithPrefix(value, symbol, prefixSystem, decimals);
}

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

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

  switch (true) {
    case absMs < 0.001:
      return `${sign}${(absMs * 1_000_000).toFixed(decimals)}ns`;
    case absMs < 1:
      return `${sign}${(absMs * 1_000).toFixed(decimals)}µs`;
    case absMs < MS_PER_SECOND:
      return `${sign}${absMs.toFixed(decimals)}ms`;
    case absMs < MS_PER_MINUTE:
      return `${sign}${(absMs / MS_PER_SECOND).toFixed(decimals)}s`;
    case absMs < MS_PER_HOUR:
      return `${sign}${(absMs / MS_PER_MINUTE).toFixed(decimals)}min`;
    case absMs < MS_PER_DAY:
      return `${sign}${(absMs / MS_PER_HOUR).toFixed(decimals)}h`;
    default:
      return `${sign}${(absMs / MS_PER_DAY).toFixed(decimals)}d`;
  }
}

/**
 * Format a duration with precision automatically derived from the visible time window.
 * Picks enough decimal places so that values ~1/1000th of the window apart
 * produce distinct formatted strings.
 * @param ms - Duration in milliseconds
 * @param windowMs - Visible time window width in milliseconds
 */
export function formatDurationForWindow(ms: number, windowMs: number): string {
  const absMs = Math.abs(ms);
  const resolution = Math.abs(windowMs) / 1000;

  let unitMs: number;
  if (absMs < 0.001) unitMs = 1e-6;
  else if (absMs < 1) unitMs = 0.001;
  else if (absMs < MS_PER_SECOND) unitMs = 1;
  else if (absMs < MS_PER_MINUTE) unitMs = MS_PER_SECOND;
  else if (absMs < MS_PER_HOUR) unitMs = MS_PER_MINUTE;
  else if (absMs < MS_PER_DAY) unitMs = MS_PER_HOUR;
  else unitMs = MS_PER_DAY;

  const resolutionInUnit = resolution / unitMs;
  const decimals =
    resolutionInUnit > 0 ? Math.min(6, Math.max(0, Math.ceil(-Math.log10(resolutionInUnit)))) : 2;

  return formatDuration(ms, decimals);
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

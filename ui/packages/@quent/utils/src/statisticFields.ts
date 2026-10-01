// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { isStatStruct, type Statistic } from './dagTypes';

type FieldPath = Array<[string, number]>;

function fieldKey(path: FieldPath): string {
  const [name, occurrence] = path[0];
  return path.length === 1 && occurrence === 0 && !name.startsWith('[')
    ? name
    : JSON.stringify(path);
}

function decodeField(field: string): FieldPath {
  if (field.startsWith('[')) {
    try {
      const parsed: unknown = JSON.parse(field);
      if (
        Array.isArray(parsed) &&
        parsed.length &&
        parsed.every(
          part =>
            Array.isArray(part) &&
            part.length === 2 &&
            typeof part[0] === 'string' &&
            Number.isInteger(part[1]) &&
            part[1] >= 0
        )
      ) {
        return parsed;
      }
    } catch {
      /* Invalid external selection is displayed literally. */
    }
  }
  return [[field, 0]];
}

export function statisticFieldLabel(field: string): string {
  return decodeField(field)
    .map(([name, occurrence]) => `${name}${occurrence ? ` [${occurrence + 1}]` : ''}`)
    .join(' › ');
}

export function statisticFieldName(field: string): string {
  const path = decodeField(field);
  return path[path.length - 1][0];
}

/** Scalar metric projection only; the inspection tree stays ordered and intact. */
export function flattenStatistics(
  statistics: readonly Statistic[],
  parent: FieldPath = []
): Statistic[] {
  const occurrences = new Map<string, number>();
  return statistics.flatMap(statistic => {
    const occurrence = occurrences.get(statistic.key) ?? 0;
    occurrences.set(statistic.key, occurrence + 1);
    const path: FieldPath = [...parent, [statistic.key, occurrence]];
    if (isStatStruct(statistic.value)) {
      return flattenStatistics(statistic.value.fields, path);
    }
    if (Array.isArray(statistic.value)) {
      return [];
    }
    return [{ ...statistic, key: fieldKey(path) }];
  });
}

/** Widths encode nonnegative volume on a logarithmic scale over the full displayed DAG. */
export function normalizeEdgeWidth(value: number, min: number, max: number): number {
  if (max <= min) {
    return 0.5;
  }
  const clamped = Math.min(max, Math.max(min, value));
  return (Math.log1p(clamped) - Math.log1p(min)) / (Math.log1p(max) - Math.log1p(min));
}

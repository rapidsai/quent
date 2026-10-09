// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { isStatStruct, type Statistic } from './dagTypes';

export type StatisticFieldPath = ReadonlyArray<readonly [string, number]>;

/** A selectable scalar statistic, with identity separate from display metadata. */
export interface StatisticField extends Statistic {
  path: StatisticFieldPath;
}

const FIELD_PATH_PREFIX = '__quent_stat_path__:';

/** Flat names stay readable; structured or repeated fields use their explicit path. */
export function statisticFieldId(path: StatisticFieldPath): string {
  const [name, occurrence] = path[0];
  if (
    path.length === 1 &&
    occurrence === 0 &&
    name.length > 0 &&
    !name.startsWith(FIELD_PATH_PREFIX)
  ) {
    return name;
  }
  return `${FIELD_PATH_PREFIX}${JSON.stringify(path)}`;
}

export function statisticFieldLabel(field: StatisticField | string): string {
  if (typeof field === 'string') {
    return field;
  }
  return field.path
    .map(([name, occurrence]) => {
      // Quote literal separators and occurrence suffixes so labels remain distinct.
      const label = !name || /[›[\]"\\\n\r]/u.test(name) ? JSON.stringify(name) : name;
      return `${label}${occurrence ? ` [${occurrence + 1}]` : ''}`;
    })
    .join(' › ');
}

export function statisticFieldName(
  field: StatisticField | string | null | undefined,
  fields?: readonly StatisticField[]
): string {
  if (field == null) {
    return '';
  }
  if (typeof field !== 'string') {
    return field.path[field.path.length - 1][0];
  }
  const resolvedField = fields?.find(candidate => candidate.key === field);
  return resolvedField ? statisticFieldName(resolvedField) : field;
}

/** Scalar metric projection only; the inspection tree stays ordered and intact. */
export function flattenStatistics(
  statistics: readonly Statistic[],
  parent: StatisticFieldPath = []
): StatisticField[] {
  const occurrences = new Map<string, number>();
  return statistics.flatMap(statistic => {
    const occurrence = occurrences.get(statistic.key) ?? 0;
    occurrences.set(statistic.key, occurrence + 1);
    const path: StatisticFieldPath = [...parent, [statistic.key, occurrence]];
    if (isStatStruct(statistic.value)) {
      return flattenStatistics(statistic.value.fields, path);
    }
    if (Array.isArray(statistic.value)) {
      return [];
    }
    return [{ ...statistic, key: statisticFieldId(path), path }];
  });
}

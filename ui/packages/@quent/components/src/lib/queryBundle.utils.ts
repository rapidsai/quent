// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  EntityRefKey,
  aggregateToNumber,
  isNumericValue,
  unwrapTaggedValue,
  type AggMode,
} from '@quent/utils';
import { QueryEntities, Operator } from '@quent/utils';
import { StatValue } from '../services/query-plan/types';

// Maps entity ref string to a key in the entities object.
// Application entities have no corresponding collection in QueryEntities, so they are omitted.
export const ENTITY_REF_TO_ENTITIES_KEY: Partial<Record<EntityRefKey, keyof QueryEntities>> = {
  Engine: 'engine',
  QueryGroup: 'query_group',
  Query: 'query',
  Plan: 'plans',
  Worker: 'workers',
  Operator: 'operators',
  Port: 'ports',
  ResourceGroup: 'resource_groups',
  Resource: 'resources',
} as const;

/**
 * Converts an EntityRef to the corresponding key in the QueryEntities object.
 * Returns undefined for entity types with no QueryEntities collection (e.g. Application).
 */
export function entityRefToEntitiesKey(entityRef: EntityRefKey): keyof QueryEntities | undefined {
  return ENTITY_REF_TO_ENTITIES_KEY[entityRef];
}

export function parseCustomStatistics(
  rawNode: unknown
): Array<{ key: string; value: StatValue; quantity?: string }> {
  const statistics = (rawNode as Operator)?.statistics?.custom_statistics;
  if (!statistics) {
    return [];
  }

  return Object.entries(statistics).map(([key, statistic]) => {
    const { value, quantity } = statistic;
    return {
      key,
      value: value ? unwrapTaggedValue(value) : null,
      ...(quantity !== null ? { quantity } : {}),
    };
  });
}

export interface ResolvedOperatorStat {
  value: StatValue;
  quantity?: string;
}

/**
 * Resolves a node's value for a statistic. A node's own statistic wins; a
 * node that groups other operators (e.g. a logical-plan node) has none, so
 * its numeric value is aggregated from the related operators with `aggMode`
 * (the same rule the pivot table column hover uses). Non-numeric statistics
 * are never aggregated.
 */
export function resolveOperatorStat(
  rawNode: unknown,
  relatedOperators: readonly unknown[] | undefined,
  field: string,
  aggMode: AggMode = 'sum'
): ResolvedOperatorStat | undefined {
  const own = parseCustomStatistics(rawNode).find(s => s.key === field);
  if (own?.value != null) {
    return {
      value: own.value,
      ...(own.quantity !== undefined ? { quantity: own.quantity } : {}),
    };
  }
  const related = (relatedOperators ?? []).flatMap(operator => {
    const stat = parseCustomStatistics(operator).find(s => s.key === field);
    return stat?.value != null && isNumericValue(stat.value) ? [stat] : [];
  });
  const value = aggregateToNumber(
    related.map(s => s.value as number | bigint),
    aggMode
  );
  if (value === undefined) {
    return undefined;
  }
  const quantity = related[0]?.quantity;
  const hasConsistentQuantity = related.every(stat => stat.quantity === quantity);
  return {
    value,
    ...(hasConsistentQuantity && quantity !== undefined ? { quantity } : {}),
  };
}

export function parsePortStatistics(rawPort: unknown): Array<{ key: string; value: StatValue }> {
  const port = rawPort as Record<string, unknown> | undefined;
  const statistics = port?.statistics as
    { custom_statistics?: Record<string, unknown> } | undefined;
  const custom = statistics?.custom_statistics;
  if (!custom) {
    return [];
  }

  return Object.entries(custom).map(([key, tagged]) => ({
    key,
    value: tagged
      ? unwrapTaggedValue(Object.values(tagged as unknown as Record<string, unknown>)[0])
      : null,
  }));
}

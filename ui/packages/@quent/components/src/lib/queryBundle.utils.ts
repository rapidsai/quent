// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  aggregateToNumber,
  isNumericValue,
  unwrapTaggedValue,
  type AggMode,
  type EntityRefKey,
  type QueryEntities,
  type Operator,
  type Port,
  type Statistic,
  type StatisticField,
} from '@quent/utils';

// Maps entity ref string to a key in the entities object.
// Application entities have no corresponding collection in QueryEntities, so they are omitted.
export const ENTITY_REF_TO_ENTITIES_KEY: Partial<Record<EntityRefKey, keyof QueryEntities>> = {
  Engine: 'engine',
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

export function parseCustomStatistics(rawNode: unknown): Statistic[] {
  return ((rawNode as Operator)?.statistics?.custom_statistics ?? []).map(
    ({ value: { key, value }, quantity }) => ({
      key,
      value: unwrapTaggedValue(value),
      ...(quantity !== null && quantity !== undefined ? { quantity } : {}),
    })
  );
}

/**
 * Resolves a node's value for a statistic field. A node's own statistic wins;
 * a node that groups other operators (e.g. a logical-plan node) has none, so
 * its numeric value is aggregated from the related operators with `aggMode`
 * (the same rule the pivot table column hover uses). Non-numeric statistics
 * are never aggregated.
 */
export function resolveStatisticFields(
  ownFields: readonly StatisticField[],
  relatedFields: readonly (readonly StatisticField[])[],
  field: string,
  aggMode: AggMode = 'sum'
): StatisticField | undefined {
  const own = ownFields.find(statistic => statistic.key === field);
  if (own?.value != null) {
    return own;
  }

  const related = relatedFields.flatMap(fields => {
    const stat = fields.find(statistic => statistic.key === field);
    return stat?.value != null && isNumericValue(stat.value) ? [stat] : [];
  });
  const value = aggregateToNumber(
    related.map(s => s.value as number | bigint),
    aggMode
  );
  if (value === undefined || related.length === 0) {
    return undefined;
  }

  const { quantity, ...base } = related[0];
  const hasConsistentQuantity = related.every(stat => stat.quantity === quantity);
  return {
    ...base,
    value,
    ...(hasConsistentQuantity && quantity !== undefined ? { quantity } : {}),
  };
}

export function parsePortStatistics(rawPort: unknown): Statistic[] {
  return ((rawPort as Port)?.statistics?.custom_statistics ?? []).map(({ key, value }) => ({
    key,
    value: unwrapTaggedValue(value),
  }));
}

export function parseOperatorAttributes(rawNode: unknown): Statistic[] {
  return ((rawNode as Operator)?.custom_attributes ?? []).map(({ key, value }) => ({
    key,
    value: unwrapTaggedValue(value),
  }));
}

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { EntityRefKey } from '@/types';
import { QueryEntities } from '~quent/types/QueryEntities';
import { Operator } from '~quent/types/Operator';
import { StatValue } from '@/services/query-plan/types';

// Maps entity ref string to a key in the entities object.
// Task has no corresponding collection in QueryEntities, so it is omitted.
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
 * Returns undefined for entity types with no QueryEntities collection (e.g. Task).
 */
export function entityRefToEntitiesKey(entityRef: EntityRefKey): keyof QueryEntities | undefined {
  return ENTITY_REF_TO_ENTITIES_KEY[entityRef];
}

function unwrapToString(val: unknown): string {
  const result = unwrapTaggedValue(val);
  return Array.isArray(result) ? result.join('\n') : String(result ?? '');
}

function unwrapTaggedValue(val: unknown): StatValue {
  switch (true) {
    case val === null || val === undefined:
      return null;
    case typeof val === 'string' || typeof val === 'number' || typeof val === 'boolean':
      return val as StatValue;
    case Array.isArray(val):
      return (val as unknown[]).map(unwrapToString);
    case typeof val === 'object': {
      const obj = val as Record<string, unknown>;
      const keys = Object.keys(obj);
      // Attribute shape: { key: string, value: Value }
      if (keys.length === 2 && 'key' in obj && 'value' in obj) {
        return `${obj.key}: ${unwrapToString(obj.value)}`;
      }
      // Tagged value: { Tag: innerValue }
      if (keys.length === 1) {
        return unwrapTaggedValue(Object.values(obj)[0]);
      }
      return JSON.stringify(val);
    }
    default:
      return String(val);
  }
}

export function parseCustomStatistics(rawNode: unknown): Array<{ key: string; value: StatValue }> {
  const statistics = (rawNode as Operator)?.statistics?.custom_statistics;
  if (!statistics) return [];

  return Object.entries(statistics).map(([key, tagged]) => ({
    key,
    value: tagged
      ? unwrapTaggedValue(Object.values(tagged as unknown as Record<string, unknown>)[0])
      : null,
  }));
}

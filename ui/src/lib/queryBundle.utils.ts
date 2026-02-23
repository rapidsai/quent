import { EntityRefKey } from '@/types';
import { QueryEntities } from '~quent/types/QueryEntities';
import { StatValue, RawNodeStatistics } from '@/services/query-plan/types';

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

export function parseCustomStatistics(rawNode: unknown): Array<{ key: string; value: StatValue }> {
  const statistics = (rawNode as RawNodeStatistics)?.statistics?.custom_statistics;
  if (!statistics) return [];

  return Object.entries(statistics).map(([key, tagged]) => ({
    key,
    value: Object.values(tagged)[0] ?? null,
  }));
}

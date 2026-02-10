import { EntityRefKey } from '@/types';
import { QueryEntities } from '~quent/types/QueryEntities';

// Maps entity ref string to a key in the entities object
export const ENTITY_REF_TO_ENTITIES_KEY = {
  Engine: 'engine',
  QueryGroup: 'query_group',
  Query: 'query',
  Plan: 'plans',
  Worker: 'workers',
  Operator: 'operators',
  Port: 'ports',
  ResourceGroup: 'resource_groups',
  Resource: 'resources',
} as const satisfies Record<EntityRefKey, keyof QueryEntities>;

/**
 * Converts an EntityRef to the corresponding key in the QueryEntities object.
 */
export function entityRefToEntitiesKey(entityRef: EntityRefKey): keyof QueryEntities {
  return ENTITY_REF_TO_ENTITIES_KEY[entityRef];
}

import { EntityRefKey } from '@/types';
import { EntitiesUI } from '~quent/types/EntitiesUI';

// Maps entity ref string to a key in the entities object
const ENTITY_REF_TO_ENTITIES_KEY = {
  Engine: 'engine',
  QueryGroup: 'query_groups',
  Query: 'queries',
  Plan: 'plans',
  Worker: 'workers',
  Operator: 'operators',
  Port: 'ports',
  ResourceGroup: 'resource_groups',
  Resource: 'resources',
  CustomFsm: 'custom_fsms',
} as const satisfies Record<EntityRefKey, keyof EntitiesUI>;

/**
 * Converts an EntityRef to the corresponding key in the EntitiesUI object.
 */
export function entityRefToEntitiesKey(entityRef: EntityRefKey): keyof EntitiesUI {
  return ENTITY_REF_TO_ENTITIES_KEY[entityRef];
}

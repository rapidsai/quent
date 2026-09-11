// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  EntityRefKey,
  unwrapTaggedValue,
  type InspectedInformationGroup,
  type InspectedOperatorObservation,
  type InspectedPortRelation,
  type Operator,
  type Port,
  type PortRelation,
  type QueryEntities,
} from '@quent/utils';

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

function parseInformation(
  information:
    | NonNullable<Operator['statistics']>['information']
    | NonNullable<Port['statistics']>['information']
    | undefined
): InspectedInformationGroup[] {
  if (!information) {
    return [];
  }

  return information.map(group => ({
    heading: group.heading,
    items: group.items.map(({ attribute: { key, value }, quantity }) => ({
      key,
      value: value == null ? null : unwrapTaggedValue(value),
      ...(quantity != null ? { quantity } : {}),
    })),
  }));
}

function parsePortRelations(
  relations: readonly PortRelation[] | undefined
): InspectedPortRelation[] {
  return (relations ?? []).map(relation => ({
    portId: relation.port_id,
    role: relation.role,
  }));
}

export function parseOperatorInformation(rawNode: unknown): InspectedInformationGroup[] {
  return parseInformation((rawNode as Operator)?.statistics?.information);
}

export function parseOperatorPortRelations(rawNode: unknown): InspectedPortRelation[] {
  return parsePortRelations((rawNode as Operator)?.statistics?.port_relations);
}

export function parseOperatorObservations(rawNode: unknown): InspectedOperatorObservation[] {
  return ((rawNode as Operator)?.observations ?? []).map(observation => ({
    timeSeconds: observation.time_s,
    kind: observation.kind,
    attributes: observation.custom_attributes.map(({ key, value }) => ({
      key,
      value: value == null ? null : unwrapTaggedValue(value),
    })),
    portRelations: parsePortRelations(observation.port_relations),
  }));
}

export function parsePortInformation(rawPort: unknown): InspectedInformationGroup[] {
  return parseInformation((rawPort as Port)?.statistics?.information);
}

export function parseCustomStatistics(rawNode: unknown) {
  return parseOperatorInformation(rawNode).flatMap(group => group.items);
}

export function parsePortStatistics(rawPort: unknown) {
  return parsePortInformation(rawPort).flatMap(group => group.items);
}

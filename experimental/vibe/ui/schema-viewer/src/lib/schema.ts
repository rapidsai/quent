// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  Annotations,
  DataType,
  Entity,
  Field,
  Path,
  Record as SchemaRecord,
  Schema,
} from '@quent/schema';

import {
  FSM_CONSTRAINT,
  REF_TARGET_CONSTRAINT,
  REF_TREE_CONSTRAINT,
  RESOURCE_CONSTRAINT,
} from './constants';
import type {
  EntityGraphModel,
  EntityGraphConfig,
  EntityReference,
  FsmTopology,
  ResourceCapacity,
  ResourceConsumer,
  ResourceDefinition,
} from './types';

export function pathKey(path: Path): string {
  return [...path.namespace, path.name].join('::');
}

export function samePath(left: Path, right: Path): boolean {
  return pathKey(left) === pathKey(right);
}

export function constraintData(
  annotations: Annotations,
  name: string,
): string | null | undefined {
  const direct = annotations.constraints[name];
  if (direct) {
    return direct.data;
  }
  return Object.values(annotations.constraints).find(
    (constraint) => constraint.name === name,
  )?.data;
}

export function hasConstraint(
  annotations: Annotations,
  name: string,
): boolean {
  return (
    name in annotations.constraints ||
    Object.values(annotations.constraints).some(
      (constraint) => constraint.name === name,
    )
  );
}

export function parseConstraintPath(value: string | null | undefined): Path | null {
  if (!value) {
    return null;
  }
  const segments = value.split('::');
  const name = segments.pop();
  return name && segments.every((segment) => segment.length > 0)
    ? { namespace: segments, name }
    : null;
}

export function buildEntityGraph(schema: Schema | null): EntityGraphModel {
  if (!schema) {
    return { nodes: [], references: [] };
  }

  const records = new Map(
    schema.records.map(([path, record]) => [pathKey(path), record]),
  );
  const references: EntityReference[] = [];

  for (const [, entity] of schema.entities) {
    collectEntityReferences(entity, records, references);
  }

  return {
    nodes: schema.entities.map(([path, entity]) => {
      const resource = parseResource(entity.annotations);
      return {
        id: pathKey(path),
        path,
        eventCount: Object.keys(entity.events).length,
        referenceCount: references.filter(
          (reference) => pathKey(reference.source) === pathKey(path),
        ).length,
        fsm: parseFsm(entity) !== null,
        resource: resource?.kind === 'definition',
      };
    }),
    references,
  };
}

function collectEntityReferences(
  entity: Entity,
  records: Map<string, SchemaRecord>,
  output: EntityReference[],
): void {
  for (const event of Object.values(entity.events)) {
    for (const field of Object.values(event.payload)) {
      collectTypeReferences({
        type: field.ty,
        source: entity.path,
        event: event.name,
        fieldPath: [field.name],
        records,
        visitedRecords: new Set(),
        output,
      });
    }
  }
}

interface CollectTypeReferencesInput {
  type: DataType;
  source: Path;
  event: string;
  fieldPath: string[];
  records: Map<string, SchemaRecord>;
  visitedRecords: Set<string>;
  output: EntityReference[];
}

function collectTypeReferences(input: CollectTypeReferencesInput): void {
  const { type } = input;
  if (typeof type === 'string') {
    return;
  }
  if ('Option' in type) {
    collectTypeReferences({ ...input, type: type.Option });
    return;
  }
  if ('List' in type) {
    collectTypeReferences({ ...input, type: type.List });
    return;
  }
  if ('Record' in type) {
    collectRecordReferences(type.Record, input);
    return;
  }

  const reference = type.EntityRef;
  const target = parseConstraintPath(
    constraintData(reference.annotations, REF_TARGET_CONSTRAINT),
  );
  const tree = hasConstraint(reference.annotations, REF_TREE_CONSTRAINT);
  input.output.push({
    id: [
      pathKey(input.source),
      input.event,
      input.fieldPath.join('.'),
      input.output.length,
    ].join(':'),
    source: input.source,
    target,
    event: input.event,
    fieldPath: input.fieldPath,
    tree,
  });

  if (reference.data) {
    collectTypeReferences({
      ...input,
      type: reference.data,
      fieldPath: [...input.fieldPath, '$data'],
    });
  }
}

function collectRecordReferences(
  path: Path,
  input: CollectTypeReferencesInput,
): void {
  const key = pathKey(path);
  if (input.visitedRecords.has(key)) {
    return;
  }
  const record = input.records.get(key);
  if (!record) {
    return;
  }

  const visitedRecords = new Set(input.visitedRecords);
  visitedRecords.add(key);
  for (const field of Object.values(record.fields)) {
    collectTypeReferences({
      ...input,
      type: field.ty,
      fieldPath: [...input.fieldPath, field.name],
      visitedRecords,
    });
  }
}

export function parseFsm(entity: Entity): FsmTopology | null {
  const data = constraintData(entity.annotations, FSM_CONSTRAINT);
  if (!data) {
    return null;
  }

  try {
    const value: unknown = JSON.parse(data);
    if (!isFsmPayload(value)) {
      return null;
    }
    const explicitExit = Object.hasOwn(entity.events, 'exit') &&
      value.transitions.some((transition) => transition.target === 'exit');
    const transitions = explicitExit
      ? value.transitions.filter((transition) => transition.target !== 'exit')
      : value.transitions;
    const states = Object.keys(entity.events).filter(
      (state) => !explicitExit || state !== 'exit',
    );
    const transitionSources = new Set(
      transitions.map((transition) => transition.source),
    );
    const exitStates = Array.from(new Set([
      ...(explicitExit
        ? value.transitions
            .filter((transition) => transition.target === 'exit')
            .map((transition) => transition.source)
        : []),
      ...states.filter((state) => !transitionSources.has(state)),
    ]));
    return {
      initialState: value.initial_state,
      transitions,
      exitStates,
      states,
    };
  } catch {
    return null;
  }
}

type ResourceRole =
  | { kind: 'definition'; capacities: ResourceCapacity[] }
  | { kind: 'usage'; resource: Path }
  | { kind: 'bounds'; resource: Path };

export function parseResource(annotations: Annotations): ResourceRole | null {
  const data = constraintData(annotations, RESOURCE_CONSTRAINT);
  if (!data) {
    return null;
  }

  try {
    const value: unknown = JSON.parse(data);
    if (!isObject(value)) {
      return null;
    }
    if (isObject(value.definition)) {
      const capacities: ResourceCapacity[] = Object.entries(
        value.definition,
      ).flatMap<ResourceCapacity>(
        ([name, capacity]) => {
          if (
            !isObject(capacity) ||
            (capacity.kind !== 'occupancy' && capacity.kind !== 'rate') ||
            typeof capacity.bounded !== 'boolean'
          ) {
            return [];
          }
          return [
            {
              name,
              kind: capacity.kind,
              bounded: capacity.bounded,
            },
          ];
        },
      );
      return { kind: 'definition', capacities };
    }
    for (const kind of ['usage', 'bounds'] as const) {
      const role = value[kind];
      if (
        isObject(role) &&
        isObject(role.resource) &&
        Array.isArray(role.resource.namespace) &&
        role.resource.namespace.every(
          (segment) => typeof segment === 'string',
        ) &&
        typeof role.resource.name === 'string'
      ) {
        return {
          kind,
          resource: {
            namespace: role.resource.namespace,
            name: role.resource.name,
          },
        };
      }
    }
    return null;
  } catch {
    return null;
  }
}

export function buildResources(schema: Schema | null): ResourceDefinition[] {
  if (!schema) {
    return [];
  }

  const recordRoles = new Map<
    string,
    { kind: 'usage' | 'bounds'; resource: Path; fields: string[]; record: Path }
  >();
  for (const [, record] of schema.records) {
    const role = parseResource(record.annotations);
    if (role?.kind === 'usage' || role?.kind === 'bounds') {
      recordRoles.set(pathKey(record.path), {
        ...role,
        record: record.path,
        fields: Object.keys(record.fields),
      });
    }
  }

  const consumers = collectResourceConsumers(schema, recordRoles);
  return schema.entities.flatMap(([, entity]) => {
    const role = parseResource(entity.annotations);
    if (role?.kind !== 'definition') {
      return [];
    }
    const related = Array.from(recordRoles.values()).filter((record) =>
      samePath(record.resource, entity.path),
    );
    return [
      {
        resource: entity.path,
        capacities: role.capacities,
        usages: related
          .filter((record) => record.kind === 'usage')
          .map((record) => ({
            record: record.record,
            fields: record.fields,
            consumers: consumers.get(pathKey(record.record)) ?? [],
          })),
        bounds: related
          .filter((record) => record.kind === 'bounds')
          .map((record) => ({
            record: record.record,
            fields: record.fields,
            consumers: consumers.get(pathKey(record.record)) ?? [],
          })),
      },
    ];
  });
}

function collectResourceConsumers(
  schema: Schema,
  roles: Map<string, unknown>,
): Map<string, ResourceConsumer[]> {
  const output = new Map<string, ResourceConsumer[]>();
  const records = new Map(
    schema.records.map(([path, record]) => [pathKey(path), record]),
  );
  for (const [, entity] of schema.entities) {
    for (const event of Object.values(entity.events)) {
      for (const field of Object.values(event.payload)) {
        collectResourceConsumerFromType({
          type: field.ty,
          entity: entity.path,
          event: event.name,
          fieldPath: [field.name],
          records,
          roles,
          inEntityReference: false,
          visited: new Set(),
          output,
        });
      }
    }
  }
  return output;
}

interface CollectResourceConsumerInput {
  type: DataType;
  entity: Path;
  event: string;
  fieldPath: string[];
  records: Map<string, SchemaRecord>;
  roles: Map<string, unknown>;
  inEntityReference: boolean;
  visited: Set<string>;
  output: Map<string, ResourceConsumer[]>;
}

function collectResourceConsumerFromType(
  input: CollectResourceConsumerInput,
): void {
  const { type } = input;
  if (typeof type === 'string') {
    return;
  }
  if ('Option' in type) {
    collectResourceConsumerFromType({ ...input, type: type.Option });
    return;
  }
  if ('List' in type) {
    collectResourceConsumerFromType({ ...input, type: type.List });
    return;
  }
  if ('EntityRef' in type) {
    if (type.EntityRef.data) {
      collectResourceConsumerFromType({
        ...input,
        type: type.EntityRef.data,
        inEntityReference: true,
      });
    }
    return;
  }

  const recordKey = pathKey(type.Record);
  if (input.inEntityReference && input.roles.has(recordKey)) {
    const values = input.output.get(recordKey) ?? [];
    values.push({
      entity: input.entity,
      event: input.event,
      fieldPath: input.fieldPath,
    });
    input.output.set(recordKey, values);
  }
  if (input.visited.has(recordKey)) {
    return;
  }
  const record = input.records.get(recordKey);
  if (!record) {
    return;
  }
  const visited = new Set(input.visited);
  visited.add(recordKey);
  for (const field of Object.values(record.fields)) {
    collectResourceConsumerFromType({
      ...input,
      type: field.ty,
      fieldPath: [...input.fieldPath, field.name],
      visited,
    });
  }
}

interface FsmPayload {
  initial_state: string;
  transitions: Array<{ source: string; target: string }>;
}

function isFsmPayload(value: unknown): value is FsmPayload {
  if (!isObject(value) || typeof value.initial_state !== 'string') {
    return false;
  }
  if (
    !Array.isArray(value.transitions) ||
    !value.transitions.every(
      (transition) =>
        isObject(transition) &&
        typeof transition.source === 'string' &&
        typeof transition.target === 'string',
    )
  ) {
    return false;
  }
  return true;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

export function formatDataType(type: DataType): string {
  if (typeof type === 'string') {
    return type;
  }
  if ('Option' in type) {
    return `Option<${formatDataType(type.Option)}>`;
  }
  if ('List' in type) {
    return `List<${formatDataType(type.List)}>`;
  }
  if ('Record' in type) {
    return pathKey(type.Record);
  }
  const target = parseConstraintPath(
    constraintData(type.EntityRef.annotations, REF_TARGET_CONSTRAINT),
  );
  const data = type.EntityRef.data
    ? `, ${formatDataType(type.EntityRef.data)}`
    : '';
  return `EntityRef<${target ? pathKey(target) : 'any'}${data}>`;
}

export interface DataTypePart {
  kind:
    | 'syntax'
    | 'type'
    | 'reference'
    | 'reference-label'
    | 'reference-target'
    | 'none';
  value: string;
}

export function dataTypeParts(type: DataType): DataTypePart[] {
  const parts: DataTypePart[] = [];
  collectDataTypeParts(type, parts);
  return parts;
}

function collectDataTypeParts(type: DataType, parts: DataTypePart[]): void {
  if (typeof type === 'string') {
    parts.push({ kind: 'type', value: type });
    return;
  }
  if ('Option' in type) {
    parts.push({ kind: 'syntax', value: 'Option<' });
    collectDataTypeParts(type.Option, parts);
    parts.push({ kind: 'syntax', value: '>' });
    return;
  }
  if ('List' in type) {
    parts.push({ kind: 'syntax', value: 'List<' });
    collectDataTypeParts(type.List, parts);
    parts.push({ kind: 'syntax', value: '>' });
    return;
  }
  if ('Record' in type) {
    parts.push({ kind: 'type', value: pathKey(type.Record) });
    return;
  }
  const target = parseConstraintPath(
    constraintData(type.EntityRef.annotations, REF_TARGET_CONSTRAINT),
  );
  parts.push(
    { kind: 'reference', value: 'Ref' },
    { kind: 'reference-label', value: 'target:' },
    {
      kind: 'reference-target',
      value: target ? pathKey(target) : 'any',
    },
    { kind: 'reference-label', value: 'data:' },
  );
  if (type.EntityRef.data) {
    collectDataTypeParts(type.EntityRef.data, parts);
  } else {
    parts.push({ kind: 'none', value: 'none' });
  }
}

export function fieldsOf(
  fields: Record<string, Field>,
): Array<{ name: string; type: string; ty: DataType }> {
  return Object.values(fields).map((field) => ({
    name: field.name,
    type: formatDataType(field.ty),
    ty: field.ty,
  }));
}

export function referenceLabel(reference: EntityReference): string {
  return `${reference.event}.${reference.fieldPath.join('.')}`;
}

export function referenceMatchesFilter(
  reference: EntityReference,
  filter: NonNullable<EntityGraphConfig['references']>,
): boolean {
  if (filter === 'all') return true;
  if (filter === 'tree') return reference.tree;
  if (filter === 'typed') return reference.target !== null;
  return reference.target === null;
}

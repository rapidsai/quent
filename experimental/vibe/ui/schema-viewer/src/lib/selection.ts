// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { Path } from '@quent/schema';

import { samePath } from './schema';
import type { SchemaSelection } from './types';

export function selectionEntity(
  selection: SchemaSelection | null,
): Path | null {
  if (!selection) {
    return null;
  }
  if (
    selection.kind === 'entity' ||
    selection.kind === 'event' ||
    selection.kind === 'fsm-state'
  ) {
    return selection.entity;
  }
  if (selection.kind === 'reference') {
    return selection.reference.source;
  }
  if (
    selection.kind === 'resource' ||
    selection.kind === 'resource-record'
  ) {
    return selection.resource;
  }
  return null;
}

export function selectionMatches(
  selection: SchemaSelection | null,
  value: SchemaSelection,
): boolean {
  if (!selection || selection.kind !== value.kind) {
    return false;
  }
  switch (value.kind) {
    case 'entity':
      return (
        selection.kind === 'entity' &&
        samePath(selection.entity, value.entity)
      );
    case 'record':
      return (
        selection.kind === 'record' &&
        samePath(selection.record, value.record)
      );
    case 'event':
      return (
        selection.kind === 'event' &&
        samePath(selection.entity, value.entity) &&
        selection.event === value.event
      );
    case 'fsm-state':
      return (
        selection.kind === 'fsm-state' &&
        samePath(selection.entity, value.entity) &&
        selection.state === value.state
      );
    case 'reference':
      return (
        selection.kind === 'reference' &&
        selection.reference.id === value.reference.id
      );
    case 'resource':
      return (
        selection.kind === 'resource' &&
        samePath(selection.resource, value.resource)
      );
    case 'resource-record':
      return (
        selection.kind === 'resource-record' &&
        samePath(selection.record, value.record) &&
        samePath(selection.resource, value.resource) &&
        selection.role === value.role
      );
  }
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  Engine,
  EntityRef,
  Operator,
  Plan,
  Port,
  Query,
  QueryGroup,
  Resource,
  ResourceGroup,
  ResourceTypeDecl,
  Worker,
} from './types/index';

export type EntityTypeValue =
  | Engine
  | Operator
  | Plan
  | Port
  | Query
  | QueryGroup
  | Resource
  | ResourceGroup
  | ResourceTypeDecl
  | Worker;

export type SingleEntity = Engine | Query | QueryGroup;

type KeysOfUnion<T> = T extends T ? keyof T : never;

export type EntityRefKey = KeysOfUnion<EntityRef>;

export interface EntityRefParts {
  variant: EntityRefKey;
  typeName: string;
  id: string;
}

export function unpackEntityRef(ref: EntityRef): EntityRefParts {
  const [variant, value] = Object.entries(ref)[0] as [
    EntityRefKey,
    string | { type_name: string; id: string },
  ];

  if (variant === 'Application') {
    const { type_name: typeName, id } = value as { type_name: string; id: string };
    return { variant, typeName, id };
  }

  return { variant, typeName: variant, id: value as string };
}

export const EntityTypeKey = {
  Resource: 'Resource',
  ResourceGroup: 'ResourceGroup',
} as const;

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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

export type EntityRefKey = keyof EntityRef;

export const EntityTypeKey = {
  Resource: 'Resource',
  ResourceGroup: 'ResourceGroup',
} as const;

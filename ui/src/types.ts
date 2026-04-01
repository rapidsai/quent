// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Top level types, keep types with relevant code where possible

import { Engine, EntityRef, Operator, Plan, Port, Query, QueryGroup, Resource, ResourceGroup, ResourceTypeDecl, Worker } from '@quent/utils';

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

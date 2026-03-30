// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Top level types, keep types with relevant code where possible

import { Engine } from '~quent/types/Engine';
import { EntityRef } from '~quent/types/EntityRef';
import { Operator } from '~quent/types/Operator';
import { Plan } from '~quent/types/Plan';
import { Port } from '~quent/types/Port';
import { Query } from '~quent/types/Query';
import { QueryGroup } from '~quent/types/QueryGroup';
import { Resource } from '~quent/types/Resource';
import { ResourceGroup } from '~quent/types/ResourceGroup';
import { ResourceTypeDecl } from '~quent/types/ResourceTypeDecl';
import { Worker } from '~quent/types/Worker';

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

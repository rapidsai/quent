// Top level types, keep types with relevant code where possible

import { Engine } from '~quent/types/Engine';
import { Query } from '~quent/types/Query';
import { Operator } from '~quent/types/Operator';
import { Plan } from '~quent/types/Plan';
import { Port } from '~quent/types/Port';
import { QueryGroup } from '~quent/types/QueryGroup';
import { Resource } from '~quent/types/Resource';
import { ResourceGroup } from '~quent/types/ResourceGroup';
import { Worker } from '~quent/types/Worker';
import { EntityRef } from '~quent/types/EntityRef';
import { DynamicFsmStateDecl } from '~quent/types/DynamicFsmStateDecl';
import { ResourceTypeDecl } from '~quent/types/ResourceTypeDecl';

export type EntityTypeValue =
  | Engine
  | DynamicFsmStateDecl
  | Operator
  | Plan
  | Port
  | Query
  | QueryGroup
  | Resource
  | ResourceGroup
  | ResourceTypeDecl
  | Worker;

export type EntityRefKey = keyof EntityRef;

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { EntityTypeValue } from '@quent/utils';
import { LucideIcon } from 'lucide-react';

export type TreeTableItem<TEntity = EntityTypeValue> = {
  id: string;
  type: string;
  entity: TEntity;
  icon?: LucideIcon;
  children?: TreeTableItem<TEntity>[];
  availableResourceTypes?: string[];
};

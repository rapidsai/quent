// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { EntityTypeValue } from '@/types';
import { LucideIcon } from 'lucide-react';

export type TreeTableItem = {
  id: string;
  type: string;
  entity: EntityTypeValue;
  icon: LucideIcon;
  children?: TreeTableItem[];
  availableResourceTypes?: string[];
};

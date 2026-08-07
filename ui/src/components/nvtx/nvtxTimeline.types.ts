// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import type { TreeTableItem } from '@quent/components';
import type { NvtxLane } from '@quent/utils';

export const NVTX_ROOT_ROW_TYPE = 'nvtx-root';
export const NVTX_DOMAIN_ROW_TYPE = 'nvtx-domain';
export const NVTX_LANE_ROW_TYPE = 'nvtx-lane';
export const NVTX_STATUS_ROW_TYPE = 'nvtx-status';

export type NvtxMetadata =
  | { kind: 'root'; label: string }
  | { kind: 'domain'; label: string; color: string }
  | { kind: 'lane'; label: string; lanes: NvtxLane[] }
  | {
      kind: 'status';
      state: 'loading' | 'empty' | 'error';
      label: string;
      retry?: () => void;
    };

export type NvtxTimelineTreeItem = TreeTableItem & {
  nvtx?: NvtxMetadata;
  children?: NvtxTimelineTreeItem[];
};

export interface NvtxTimelinePlacement {
  parentId: string;
  item: NvtxTimelineTreeItem;
}

export interface NvtxTimelineAdapter {
  placements: NvtxTimelinePlacement[];
  initiallyExpandedIds: string[];
  controls: ReactNode;
}

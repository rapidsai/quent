// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// DAG hooks
export { useSelectedNodeIds, useSetSelectedNodeIds } from './dag/useSelectedNodeIds';
export { useSelectedOperatorLabel, useSetSelectedOperatorLabel } from './dag/useSelectedOperatorLabel';
export { useSelectedPlanId, useSetSelectedPlanId } from './dag/useSelectedPlanId';
export { useHoveredWorkerId, useSetHoveredWorkerId } from './dag/useHoveredWorkerId';

// Timeline hooks
export {
  useTimelineData,
  useIsTimelineHovered,
  useZoomRange,
  useSetZoomRange,
  useDebouncedZoomRange,
  useSetDebouncedZoomRange,
  useHoveredTimelineId,
  useSetHoveredTimelineId,
  useStartTimeMs,
  useSetStartTimeMs,
  useBulkInitialized,
  useSetBulkInitialized,
  useVisibleEntries,
  useSetVisibleEntries,
  useHideTasks,
  useSetHideTasks,
  useHydrateTimelineAtoms,
} from './timeline/useTimelineAtoms';

// Timeline cache key helper (consumers need this to address per-item data)
export { timelineCacheKey } from './atoms/timeline';
export type { TimelineCacheParams } from './atoms/timeline';

// Complex timeline hooks
export { useBulkTimelines } from './timeline/useBulkTimelines';
export type { TreeNode } from './timeline/useBulkTimelines';
export {
  useBulkTimelineFetch,
  applyBulkTimelineResponse,
  buildMergedBulkEntries,
} from './timeline/useBulkTimelineFetch';
export type { BulkTimelineIdMeta, MergedBulkEntries } from './timeline/useBulkTimelineFetch';

// Highlighted items hook
export { useHighlightedItemIds } from './timeline/useHighlightedItemIds';

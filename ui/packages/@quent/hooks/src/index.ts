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

// DAG controls hooks (computation functions injected to avoid circular dep with @quent/components)
export {
  useDagNodeColoring,
  useDagEdgeWidthConfig,
  useDagEdgeColoring,
  useOperatorStatFields,
  usePortStatFields,
} from './dag/useDagControls';

// DAG node coloring hook (accepts isDark instead of useTheme for decoupling)
export { useNodeColoring } from './dag/useNodeColoring';

// DAG control selector hooks (wrapping private atoms per HOOKS-02)
export {
  useSelectedColorField,
  useNodeColoringValue,
  useSetNodeColoring,
  useNodeColorPalette,
  useSelectedEdgeWidthField,
  useEdgeWidthConfig,
  useSelectedEdgeColorField,
  useEdgeColoring,
  useEdgeColorPalette,
  useSelectedNodeLabelField,
  useHoveredNodeData,
  useSetHoveredNodeData,
  useSelectedNodeData,
  useSetSelectedNodeData,
  useHighlightedNodeIds,
  useSetHighlightedNodeIds,
  useEffectiveHighlightedNodeIds,
  useEffectiveHoveredStat,
  useHoveredStat,
  useSetHoveredStat,
  useSetDagDisplayedNodeIds,
} from './dag/dagControlSelectors';
export type {
  HoveredStatInfo,
  HighlightedNodeIdsState,
  InspectedNodeData,
} from './atoms/dagControls';

// Utility hooks
export { useDeferredReady } from './dag/useDeferredReady';

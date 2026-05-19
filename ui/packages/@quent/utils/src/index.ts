// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Utilities
export { cn } from './cn';
export { parseJsonWithBigInt } from './parseJsonWithBigInt';

// Color utilities
export {
  PALETTES,
  getColorForKey,
  assignColors,
  getColorByIndex,
  getOperationTypeColor,
  buildOperatorColorMap,
  withOpacity,
  resetColorAssignments,
  darkenColor,
  getActivePalette,
  setActivePalette,
  getPalette,
  BLACK,
  WHITE,
  isLightColor,
  createCapacitiesColorFn,
  createFsmTypeColorFn,
  CONTINUOUS_PALETTES,
  continuousColor,
  getLegendGradientStops,
} from './colors';
export type { PaletteName, PaletteTheme, ChartColor, ContinuousPaletteName } from './colors';

// Formatter utilities
export {
  formatDuration,
  formatDurationForWindow,
  formatDurationForAxisInterval,
  formatQuantity,
  formatBytes,
  formatNumber,
  inferFieldFormatter,
} from './formatters';

// Rust-generated TypeScript types
export * from './types/index';

// Timeline types and constants
export type { ZoomRange } from './types/ZoomRange';
export const MAX_TIMELINE_BINS = 200;

// Entity types (from ui/src/types.ts)
export { EntityTypeKey } from './entityTypes';
export type { EntityTypeValue, SingleEntity, EntityRefKey } from './entityTypes';

// DAG coloring types (shared between @quent/hooks and @quent/components)
export { NODE_LABEL_FIELD } from './dagTypes';
export type {
  ContinuousNodeColoring,
  CategoricalNodeColoring,
  NodeColoring,
  EdgeWidthConfig,
  ContinuousEdgeColoring,
  CategoricalEdgeColoring,
  EdgeColoring,
  NodeLabelField,
  StatValue,
  DAGNode,
  DAGEdge,
} from './dagTypes';

// Operator timeline row ID utilities
export const OPERATOR_TIMELINE_ROW_TYPE = 'operator-timeline';
const OPERATOR_TIMELINE_ROW_ID_PREFIX = '__operator_timeline__';
export function operatorTimelineRowId(workerId: string): string {
  return `${OPERATOR_TIMELINE_ROW_ID_PREFIX}${workerId}`;
}
export function workerIdFromOperatorTimelineRowId(id: string): string | null {
  return id.startsWith(OPERATOR_TIMELINE_ROW_ID_PREFIX)
    ? id.slice(OPERATOR_TIMELINE_ROW_ID_PREFIX.length)
    : null;
}

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// ─── UI primitives ────────────────────────────────────────────────────────────
export { Button, buttonVariants } from './ui/button';
export type { ButtonProps } from './ui/button';
export { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent } from './ui/card';
export { Collapsible, CollapsibleTrigger, CollapsibleContent } from './ui/collapsible';
export { DataText } from './ui/data-text';
export {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuCheckboxItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuGroup,
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from './ui/dropdown-menu';
export { HoverCard, HoverCardTrigger, HoverCardContent } from './ui/hover-card';
export { Input } from './ui/input';
export {
  navigationMenuTriggerStyle,
  NavigationMenu,
  NavigationMenuList,
  NavigationMenuItem,
  NavigationMenuContent,
  NavigationMenuTrigger,
  NavigationMenuLink,
  NavigationMenuIndicator,
  NavigationMenuViewport,
} from './ui/navigation-menu';
export { Popover, PopoverTrigger, PopoverContent } from './ui/popover';
export { ResizablePanelGroup, ResizablePanel, ResizableHandle } from './ui/resizable';
export { ScrollArea, ScrollBar } from './ui/scroll-area';
export {
  Select,
  SelectGroup,
  SelectValue,
  SelectTrigger,
  SelectContent,
  SelectLabel,
  SelectItem,
  SelectSeparator,
  SelectScrollUpButton,
  SelectScrollDownButton,
} from './ui/select';
export { SelectField } from './ui/select-field';
export type { SelectFieldProps, SelectFieldOption } from './ui/select-field';
export { Skeleton } from './ui/skeleton';
export { TreeView } from './ui/tree-view';
export type { TreeDataItem } from './ui/tree-view';
export { TreeTable } from './ui/tree-table';
export type { Column, ColumnComponent, IconComponent } from './ui/tree-table';
export { Badge, badgeVariants } from './ui/badge';
export { OptionMultiSelect } from './ui/option-multi-select';
export {
  Table,
  TableHeader,
  TableBody,
  TableFooter,
  TableHead,
  TableRow,
  TableCell,
  TableCaption,
} from './ui/table';

// ─── ECharts ──────────────────────────────────────────────────────────────────
export { echarts } from './lib/echarts';
export type { EChartsOption } from './lib/echarts';

// ─── Lib utilities ────────────────────────────────────────────────────────────
export {
  entityRefToEntitiesKey,
  ENTITY_REF_TO_ENTITIES_KEY,
  parseCustomStatistics,
  parsePortStatistics,
} from './lib/queryBundle.utils';
export { getIconForType, collectResourceTypesFromTree } from './lib/resource.utils';
export {
  nanosToMs,
  connectChart,
  registerAxisPointerSync,
  unregisterAxisPointerSync,
  buildBinnedTimelineSeries,
  buildBulkParamsForItem,
  buildTimelineMarks,
  collectVisibleEntries,
  getAdaptiveNumBins,
  getChartGroupZoomState,
  getFsmTypeName,
  getLongEntitiesThreshold,
  getLongFsms,
  getResourceTypeName,
  getTimelineConfig,
  getTimelineXAxisIntervalMs,
  mergeOverlaySeries,
  setOperatorOnEntries,
  setOperatorOnEntry,
  findItemById,
  transformResourceTree,
} from './lib/timeline.utils';
export type { AxisPointerSyncOptions } from './lib/timeline.utils';

// ─── Services – query-plan ────────────────────────────────────────────────────
export {
  computeNodeColoring,
  computeEdgeColoring,
  computeEdgeWidthConfig,
  inferFieldFormatter,
} from './services/query-plan/dagFieldProcessing';
export {
  DEFAULT_OPERATION_COLOR,
  OPERATION_TYPE_COLORS,
  getOperatorColor,
} from './services/query-plan/operationTypes';
export {
  getPlanDAG,
  getTreeData,
  validateQueryBundle,
} from './services/query-plan/query-bundle-transformer';
export type { DAGData, QueryPlanDataItem, QueryPlanNodeData } from './services/query-plan/types';
// DAGNode, DAGEdge, StatValue re-exported via services/query-plan/types (avoid direct @quent/utils re-export here)

// ─── Timeline components ──────────────────────────────────────────────────────
export { CHART_GROUP } from './timeline/Timeline';
export { Timeline } from './timeline/Timeline';
export { TimelineController } from './timeline/TimelineController';
export { TimelineSkeleton } from './timeline/TimelineSkeleton';
export { TimelineToolbar } from './timeline/TimelineToolbar';
export { QueryToolbar } from './timeline/QueryToolbar';
export { TooltipContent } from './timeline/TimelineTooltip';
export {
  useTimelineEchartsTheme,
  TIMELINE_MONO_FONT,
  TIMELINE_THEME_NAME_LIGHT,
  TIMELINE_THEME_NAME_DARK,
  MARK_AREA_BORDER_OPACITY,
  MARK_AREA_FILL_OPACITY,
  MARK_LABEL_TEXT_COLOR,
  ROLLUP_TIMELINE_COLOR_LIGHT,
  ROLLUP_TIMELINE_COLOR_DARK,
} from './timeline/timelineEchartsTheme';
export {
  DEFAULT_TIMELINE_HEIGHT,
  TIMELINE_SPACING,
  TIMELINE_X_AXIS_ANIMATION,
} from './timeline/types';
export type { TimelineMark, TimelineSeries, TimelineSeriesEntry } from './timeline/types';
export { ResourceTimeline } from './timeline/ResourceTimeline';

// ─── DAG components ───────────────────────────────────────────────────────────
export { DAGChart } from './dag/DAGChart';
export { DAGControls } from './dag/DAGControls';
export { DAGLegend } from './dag/DAGLegend';
export { DAGNodeInfoPanel } from './dag/DAGNodeInfoPanel';
export { DAGSettingsPopover } from './dag/DAGSettingsPopover';

// ─── Query-plan components ────────────────────────────────────────────────────
export { QueryPlanNode } from './query-plan/QueryPlanNode';

// ─── Resource-tree components ─────────────────────────────────────────────────
export { InlineSelector } from './resource-tree/InlineSelector';
export { ResourceColumn } from './resource-tree/ResourceColumn';
export { ResourceGroupRow } from './resource-tree/ResourceGroupRow';
export { ResourceRow } from './resource-tree/ResourceRow';
export type { TreeTableItem } from './resource-tree/types';
export { UsageColumn } from './resource-tree/UsageColumn';

// ─── Pivot-table components ──────────────────────────────────────────────────
export { GroupedDataTable } from './pivot-table/GroupedDataTable';
export type {
  GroupedDataTableProps,
  GroupedDataTableVirtualizationOptions,
  GroupedDataTableGroupRenderMode,
} from './pivot-table/GroupedDataTable';
export { PivotedStatTable } from './pivot-table/PivotedStatTable';
export { PivotTableToolbar } from './pivot-table/PivotTableToolbar';
export type { IndexConfigEntry, PivotTableToolbarProps } from './pivot-table/PivotTableToolbar';
export type {
  AggMode,
  HoveredStatInfo,
  GroupedDataTableRowBase,
  GroupedDataTableSortInfo,
  GroupedDataTableGroupKeyEntry,
  DataHeaderProps,
  GroupCellProps,
  DataCellProps,
  SortDir,
  StatGroupInputGroupValue,
  StatGroupExpandedRow,
  PivotedStatTableSchema,
  GroupKeyEntry,
  PivotedRowAgg,
  PivotedRow,
} from './pivot-table/types';
export {
  buildPivotedRows,
  computeRowSpans,
  expandRowsFromSchema,
  formatNumericStat,
  formatStatValue,
  getGroupKeys,
  getSchemaStatNames,
  getSortValue,
  getUniqueStatNames,
  gradientBg,
  isNumericValue,
  itemHasId,
  rowGroupKey,
} from './pivot-table/utils';
export type { GroupIndexDef, RowWithGroupKeys } from './pivot-table/utils';

// ─── Operator-timeline components ────────────────────────────────────────────
export { OperatorGanttChart } from './operator-timeline/OperatorGanttChart';
export type { OperatorGanttChartProps } from './operator-timeline/OperatorGanttChart';
export type { OperatorActiveSpanEntry } from './operator-timeline/types';
export {
  clipRectByRect,
  OPERATOR_TIMELINE_ROW_TYPE,
  operatorTimelineRowId,
  workerIdFromOperatorTimelineRowId,
  getWorkerIdsFromPlanTree,
  getPlanIdsForWorker,
  stackOperatorsIntoRows,
  spanToMs,
  operatorsWithActiveSpans,
  operatorsWithActiveSpansForWorker,
} from './operator-timeline/utils';

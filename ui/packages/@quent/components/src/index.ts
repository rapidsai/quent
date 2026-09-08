// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// ─── Provider ─────────────────────────────────────────────────────────────────
// Re-exported from @quent/hooks so consumers using only @quent/components have
// a single import for the runtime providers (QueryClientProvider + JotaiProvider).
export { QuentProvider } from '@quent/hooks';
export type { QuentProviderProps } from '@quent/hooks';

// ─── UI primitives ────────────────────────────────────────────────────────────
export { Button, buttonVariants } from './ui/button';
export type { ButtonProps } from './ui/button';
export { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent } from './ui/card';
export { Collapsible, CollapsibleTrigger, CollapsibleContent } from './ui/collapsible';
export { ControlField, ControlGrid, ControlSection } from './ui/control-grid';
export type { ControlFieldProps, ControlGridProps, ControlSectionProps } from './ui/control-grid';
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
export {
  Drawer,
  DrawerPortal,
  DrawerOverlay,
  DrawerTrigger,
  DrawerClose,
  DrawerContent,
  DrawerHeader,
  DrawerFooter,
  DrawerTitle,
  DrawerDescription,
} from './ui/drawer';
export { HoverCard, HoverCardTrigger, HoverCardContent } from './ui/hover-card';
export {
  OverflowHoverCardContent,
  OverflowingItemLabel,
  useOverflowHoverCard,
} from './ui/overflow-hover-card';
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
export {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from './ui/pagination';
export { Popover, PopoverTrigger, PopoverContent } from './ui/popover';
export { PointerTooltipPortal } from './ui/pointer-tooltip-portal';
export type { PointerPosition } from './ui/pointer-tooltip-portal';
export { PositionedTooltip } from './ui/positioned-tooltip';
export { ResizablePanelGroup, ResizablePanel, ResizableHandle } from './ui/resizable';
export { ScrollArea, ScrollBar } from './ui/scroll-area';
export {
  ThinScroll,
  thinScrollbarClass,
  HiddenScroll,
  hiddenScrollbarClass,
} from './ui/thin-scroll';
export type { ThinScrollProps, HiddenScrollProps } from './ui/thin-scroll';
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
export { SearchableSelect } from './ui/searchable-select';
export type { SearchableSelectProps } from './ui/searchable-select';
export { RequiredMultiSelectField } from './ui/required-multi-select-field';
export type {
  RequiredMultiSelectFieldProps,
  RequiredMultiSelectOption,
} from './ui/required-multi-select-field';
export { Skeleton } from './ui/skeleton';
export { Slider } from './ui/slider';
export { SliderField } from './ui/slider-field';
export type { SliderFieldProps } from './ui/slider-field';
export { RangeSliderField } from './ui/range-slider-field';
export type { RangeSliderFieldProps } from './ui/range-slider-field';
export { TreeView } from './ui/tree-view';
export type { TreeDataItem } from './ui/tree-view';
export { TreeTable } from './ui/tree-table';
export type { Column, ColumnComponent, IconComponent } from './ui/tree-table';
export { Badge, badgeVariants } from './ui/badge';
export { OptionMultiSelect } from './ui/option-multi-select';
export type { OptionMultiSelectOption } from './ui/option-multi-select';
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
export { Tabs, TabsList, TabsTrigger, TabsContent } from './ui/tabs';
export {
  Toaster,
  Toast,
  ToastAction,
  ToastClose,
  ToastContent,
  ToastDescription,
  ToastPortal,
  ToastProvider,
  ToastTitle,
  ToastViewport,
  createToastManager,
  toast,
  useToastManager,
} from './ui/toast';

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
  buildBinnedTimelineSeries,
  buildBulkParamsForItem,
  buildTimelineMarks,
  collectVisibleEntries,
  getAdaptiveNumBins,
  getLongEntitiesThreshold,
  getLongFsms,
  getTimelineConfig,
  getTimelineXAxisIntervalMs,
  mergeOverlaySeries,
  setOperatorOnEntries,
  setOperatorOnEntry,
  findItemById,
  transformResourceTree,
} from './lib/timeline.utils';
export { getFsmTypeName, getResourceTypeName } from '@quent/utils';

// ─── Services – query-plan ────────────────────────────────────────────────────
export {
  computeNodeColoring,
  computeEdgeColoring,
  computeEdgeWidthConfig,
} from './services/query-plan/dagFieldProcessing';
export {
  getPlanDAG,
  getTreeData,
  validateQueryBundle,
} from './services/query-plan/query-bundle-transformer';
export type { DAGData, QueryPlanDataItem, QueryPlanNodeData } from './services/query-plan/types';
// DAGNode, DAGEdge, StatValue re-exported via services/query-plan/types (avoid direct @quent/utils re-export here)

// ─── Timeline components ──────────────────────────────────────────────────────
export { TimelineController } from './timeline/TimelineController';
export { TimelinePointerArea } from './timeline/TimelinePointerArea';
export type {
  TimelinePointerAreaProps,
  TimelinePointerRange,
} from './timeline/TimelinePointerArea';
export { TimelineRuler } from './timeline/TimelineRuler';
export { TimelineSettingsPopover } from './timeline/TimelineSettingsPopover';
export { TimelineSkeleton } from './timeline/TimelineSkeleton';
export { TimelineToolbar } from './timeline/TimelineToolbar';
export { QueryToolbar } from './timeline/QueryToolbar';
export { TooltipContent } from './timeline/TimelineTooltip';
export type { TooltipItemNoun } from './timeline/TimelineTooltip';
export { TimelineTooltipPortal } from './timeline/TimelineTooltipPortal';
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
  CHART_GROUP,
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
export { DagPlayhead } from './dag/DagPlayhead';

// ─── Query-plan components ────────────────────────────────────────────────────
export { QueryPlanNode } from './query-plan/QueryPlanNode';
export { NodeFlowBar } from './query-plan/NodeFlowBar';

// ─── Segmented-bar components ─────────────────────────────────────────────────
export { SegmentedBar } from './segmented-bar/SegmentedBar';
export type { SegmentedBarProps, SegmentedBarSegment } from './segmented-bar/SegmentedBar';

// ─── Resource-tree components ─────────────────────────────────────────────────
export { InlineSelector } from './resource-tree/InlineSelector';
export type { InlineSelectorOption } from './resource-tree/InlineSelector';
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
  PivotTableInteractionConfig,
  PivotTableRenderConfig,
  PivotTableDisplayConfig,
  PivotTableDnDConfig,
  PivotTableGroupCellHoverHandlers,
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

// ─── FSM chart components ─────────────────────────────────────────────────────
export { FsmCapacityChart } from './fsm-chart/FsmCapacityChart';
export type { FsmCapacityChartProps } from './fsm-chart/FsmCapacityChart';

// ─── Long-entities components ─────────────────────────────────────────────────
export {
  LongEntitiesGantt,
  LONG_ENTITIES_TIMELINE_HEIGHT,
} from './long-entities/LongEntitiesGantt';
export type { LongEntitiesGanttProps } from './long-entities/LongEntitiesGantt';
export type { LongEntityEntry, LongEntitySegment } from './long-entities/types';
export {
  buildLongEntityEntries,
  LONG_ENTITIES_ROW_TYPE,
  longEntitiesRowId,
  resourceIdFromLongEntitiesRowId,
} from './long-entities/utils';

// ─── Operator-timeline components ────────────────────────────────────────────
export { OperatorGanttChart } from './operator-timeline/OperatorGanttChart';
export type { OperatorGanttChartProps } from './operator-timeline/OperatorGanttChart';
export type { OperatorActiveSpanEntry } from './operator-timeline/types';
export {
  OPERATOR_TIMELINE_ROW_TYPE,
  operatorTimelineRowId,
  workerIdFromOperatorTimelineRowId,
  getWorkerIdsFromPlanTree,
  getPlanIdsForWorker,
  spanToMs,
  operatorsWithActiveSpans,
  operatorsWithActiveSpansForWorker,
} from './operator-timeline/utils';
export {
  clipRectByRect,
  stackIntervalsIntoRows,
  stackIntervalsIntoRows as stackOperatorsIntoRows,
  layoutGanttBar,
  ganttExpansionLayout,
} from './gantt-chart/utils';

// ─── NVTX timeline ────────────────────────────────────────────────────────────
export { NvtxGantt, NVTX_GANTT_HEIGHT } from './nvtx-timeline/NvtxGantt';
export type { NvtxGanttProps } from './nvtx-timeline/NvtxGantt';
export {
  NVTX_SECTION_ROW_TYPE,
  NVTX_DOMAIN_ROW_TYPE,
  NVTX_LANE_ROW_TYPE,
  NVTX_SECTION_ID,
  nvtxDomainRowId,
  nvtxThreadRowId,
  nvtxProcessRowId,
  nvtxMarksRowId,
  buildNvtxTree,
  filterNvtxTree,
  indexNvtxLanes,
  isNvtxTreeEntity,
  nvtxDomainMeta,
  nvtxLaneLabel,
  nvtxDefaultExpandedIds,
} from './nvtx-timeline/utils';
export type { NvtxTreeEntity, NvtxTreeFilterResult, NvtxTreeItem } from './nvtx-timeline/utils';

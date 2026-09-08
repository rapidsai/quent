// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, type ReactNode } from 'react';
import {
  DEFAULT_TIMELINE_HEIGHT,
  TimelineController,
  TimelineRuler,
  TimelineToolbar,
  TreeTable,
  nanosToMs,
  type Column,
  type NvtxTreeEntity,
  type TreeTableItem,
} from '@quent/components';
import { useHydrateTimelineAtoms } from '@quent/hooks';
import type {
  EntityRef,
  EntityTypeValue,
  QueryBundle,
  SingleTimelineResponse,
  ZoomRange,
} from '@quent/utils';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';

export type TimelineTreeItem = TreeTableItem<EntityTypeValue | NvtxTreeEntity>;

export interface TimelineTreeModel {
  trees: TimelineTreeItem[];
  renderLabel: (item: TimelineTreeItem) => ReactNode;
  renderTimeline: (item: TimelineTreeItem) => ReactNode;
}

export interface TimelineTreeControls {
  emptyMessage?: string;
  filters?: ReactNode;
  initialSelectedItemId?: string;
  expandedIds: Set<string>;
  highlightedItemIds?: Set<string>;
  timelineData?: SingleTimelineResponse;
  onExpandChange: (itemId: string, isExpanded: boolean) => void;
  onZoomChange?: (range: ZoomRange) => void;
}

interface TimelineTreeTableProps {
  durationSeconds: number;
  isDark: boolean;
  trees: TimelineTreeModel[];
  controls: TimelineTreeControls;
  children?: ReactNode;
}

function indexTree(
  item: TimelineTreeItem,
  model: TimelineTreeModel,
  modelsByItemId: Map<string, TimelineTreeModel>
) {
  modelsByItemId.set(item.id, model);
  for (const child of item.children ?? []) {
    indexTree(child, model, modelsByItemId);
  }
}

// Standalone and combined tree containers share identical timeline setup.
// eslint-disable-next-line react-refresh/only-export-components
export function useTimelineTreeSetup(
  queryBundle: QueryBundle<EntityRef>,
  initialZoomRange?: ZoomRange
) {
  const { theme } = useTheme();
  const isDark = theme === THEME_DARK;
  const durationSeconds = queryBundle.duration_s;
  const defaultZoomRange = { start: 0, end: durationSeconds };
  const startTimeMs = useMemo(
    () => nanosToMs(queryBundle.start_time_unix_ns),
    [queryBundle.start_time_unix_ns]
  );

  useHydrateTimelineAtoms({
    zoomRange: initialZoomRange ?? defaultZoomRange,
    debouncedZoomRange: initialZoomRange ?? defaultZoomRange,
    startTimeMs,
  });

  return { durationSeconds, isDark };
}

export function TimelineTreeTable({
  durationSeconds,
  isDark,
  trees,
  controls,
  children,
}: TimelineTreeTableProps) {
  const { data, modelsByItemId } = useMemo(() => {
    const modelsByItemId = new Map<string, TimelineTreeModel>();
    const data: TimelineTreeItem[] = [];
    for (const model of trees) {
      for (const tree of model.trees) {
        data.push(tree);
        indexTree(tree, model, modelsByItemId);
      }
    }
    return { data, modelsByItemId };
  }, [trees]);
  const columns = useMemo(
    () =>
      [
        {
          key: 'resource',
          label: 'Resource',
          widthIndex: 0,
          isFirst: true,
          headerContent: (
            <div className="flex h-full select-none items-center px-3 text-xs font-semibold text-muted-foreground">
              Resource
            </div>
          ),
          render: ({ item }: { item: TimelineTreeItem }) =>
            modelsByItemId.get(item.id)?.renderLabel(item) ?? null,
        },
        {
          key: 'usage',
          label: 'Usage',
          widthIndex: 1,
          headerContent: (
            <div className="flex h-full items-center overflow-hidden py-1">
              <TimelineController
                durationSeconds={durationSeconds}
                timelineData={controls.timelineData}
                onZoomChange={controls.onZoomChange}
                isDark={isDark}
              />
            </div>
          ),
          subHeaderContent: <TimelineRuler isDark={isDark} />,
          render: ({ item }: { item: TimelineTreeItem }) =>
            modelsByItemId.get(item.id)?.renderTimeline(item) ?? null,
        },
      ] satisfies Column<TimelineTreeItem>[],
    [controls.onZoomChange, controls.timelineData, durationSeconds, isDark, modelsByItemId]
  );

  return (
    <div className="flex h-full w-full min-w-0 flex-col">
      <TimelineToolbar durationSeconds={durationSeconds} filters={controls.filters} />
      <div className="min-h-0 min-w-0 flex-1">
        {data.length > 0 ? (
          <TreeTable<TimelineTreeItem>
            data={data}
            columns={columns}
            initialSelectedItemId={controls.initialSelectedItemId}
            columnWidths={[275, 'auto']}
            onExpandChange={controls.onExpandChange}
            highlightedItemIds={controls.highlightedItemIds}
            controlledExpandedIds={controls.expandedIds}
            virtualized
            rowHeight={DEFAULT_TIMELINE_HEIGHT}
          />
        ) : (
          <div className="flex h-full items-center justify-center px-4 text-sm text-muted-foreground">
            {controls.emptyMessage ?? 'No timeline data available.'}
          </div>
        )}
      </div>
      {children}
    </div>
  );
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import {
  DEFAULT_TIMELINE_HEIGHT,
  EmptyState,
  TimelineActions,
  TimelineController,
  TimelineRuler,
  TimelineToolbar,
  PinnedSectionToggle,
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
import { useTabActionSlots } from '@/components/TabActionSlots';
import { PinnableRowLabel } from './PinnableRowLabel';
import { PinnedRowsSplit } from './PinnedRowsSplit';
import { useSyncedHorizontalScroll } from './useSyncedHorizontalScroll';

type TimelineColumnWidths = (number | 'auto')[];
const DEFAULT_COLUMN_WIDTHS: TimelineColumnWidths = [275, 'auto'];

// Keeps the space for a scrollbar on every table, so their columns line up whether or not
// any of them is currently scrolling.
const STABLE_GUTTER = '[&_[data-tree-table-scroller]]:[scrollbar-gutter:stable]';

export type TimelineTreeItem = TreeTableItem<EntityTypeValue | NvtxTreeEntity>;

export interface TimelineTreeModel {
  trees: TimelineTreeItem[];
  // The description is extra context for the row, given only for rows in the pinned section.
  renderLabel: (item: TimelineTreeItem, description?: ReactNode, isPinned?: boolean) => ReactNode;
  renderTimeline: (item: TimelineTreeItem, isPinned?: boolean) => ReactNode;
  // Which of this tree's rows can be pinned. Trees that leave it out have no pin buttons.
  rowPinning?: RowPinning;
}

export interface RowPinning {
  isPinnable: (item: TimelineTreeItem) => boolean;
  // Plain-text name of a row, used to label its pin button.
  getName: (item: TimelineTreeItem) => string;
  // Extra context shown only on the pinned copy of a row.
  getDescription?: (item: TimelineTreeItem) => ReactNode;
}

export interface PinningControls {
  // Pinned rows to show above the tree, after any filter has been applied.
  items: TimelineTreeItem[];
  // Pinned rows that a filter currently hides.
  hiddenCount: number;
  ids: ReadonlySet<string>;
  isAtLimit: boolean;
  onToggle: (itemId: string) => void;
  // Whether the pinned section shows only its header.
  isCollapsed: boolean;
  onToggleCollapsed: () => void;
  // Moves a pinned row to where another pinned row sits, when the user drags it there.
  onReorder: (activeId: string, overId: string) => void;
}

export interface TimelineTreeControls {
  emptyMessage?: string;
  filters?: ReactNode;
  initialSelectedItemId?: string;
  expandedIds: Set<string>;
  highlightedItemIds?: Set<string>;
  indicatorItemIds?: Set<string>;
  timelineData?: SingleTimelineResponse;
  onExpandChange: (itemId: string, isExpanded: boolean) => void;
  onZoomChange?: (range: ZoomRange) => void;
  // Leave out for trees with no pinning.
  pinning?: PinningControls;
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
  const slots = useTabActionSlots();
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
  const { pinning } = controls;
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
          render: ({ item, isPinned }: { item: TimelineTreeItem; isPinned?: boolean }) => {
            const model = modelsByItemId.get(item.id);
            const rowPinning = model?.rowPinning;
            return (
              <PinnableRowLabel
                label={
                  model?.renderLabel(
                    item,
                    isPinned ? rowPinning?.getDescription?.(item) : undefined,
                    isPinned
                  ) ?? null
                }
                name={rowPinning?.getName(item) ?? item.id}
                isPinnable={pinning != null && (rowPinning?.isPinnable(item) ?? false)}
                isPinned={pinning?.ids.has(item.id) ?? false}
                isAtPinLimit={pinning?.isAtLimit ?? false}
                onTogglePin={() => pinning?.onToggle(item.id)}
              />
            );
          },
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
          render: ({ item, isPinned }: { item: TimelineTreeItem; isPinned?: boolean }) =>
            modelsByItemId.get(item.id)?.renderTimeline(item, isPinned) ?? null,
        },
      ] satisfies Column<TimelineTreeItem>[],
    [controls.onZoomChange, controls.timelineData, durationSeconds, isDark, modelsByItemId, pinning]
  );

  const hasPinnedSection = pinning != null && (pinning.items.length > 0 || pinning.hiddenCount > 0);

  // The headers live in their own table above the resizable split so they never scroll away.
  // The tables below drop theirs.
  const bodyColumns = useMemo(
    () =>
      columns.map(column => ({
        ...column,
        headerContent: undefined,
        subHeaderContent: undefined,
      })),
    [columns]
  );

  // The header table and the two body tables are separate, so they share one set of widths.
  const [columnWidths, setColumnWidths] = useState<TimelineColumnWidths>(DEFAULT_COLUMN_WIDTHS);

  // Likewise, each table scrolls horizontally on its own, so keep them at the same offset.
  const tablesRef = useRef<HTMLDivElement>(null);
  useSyncedHorizontalScroll(tablesRef, data.length > 0);

  return (
    <div className="flex h-full w-full min-w-0 flex-col">
      {slots ? (
        <>
          {slots.leftSlot && controls.filters && createPortal(controls.filters, slots.leftSlot)}
          {slots.rightSlot &&
            createPortal(<TimelineActions durationSeconds={durationSeconds} />, slots.rightSlot)}
        </>
      ) : (
        <TimelineToolbar durationSeconds={durationSeconds} filters={controls.filters} />
      )}
      <div ref={tablesRef} className={`flex min-h-0 min-w-0 flex-1 flex-col ${STABLE_GUTTER}`}>
        {data.length > 0 && (
          <div className="shrink-0">
            <TreeTable<TimelineTreeItem>
              data={[]}
              columns={columns}
              columnWidths={columnWidths}
              onColumnWidthsChange={setColumnWidths}
            />
          </div>
        )}
        {data.length > 0 ? (
          <PinnedRowsSplit
            pinned={
              hasPinnedSection ? (
                <TreeTable<TimelineTreeItem>
                  data={[]}
                  columns={bodyColumns}
                  columnWidths={columnWidths}
                  onColumnWidthsChange={setColumnWidths}
                  pinnedItems={pinning.items}
                  isPinnedCollapsed={pinning.isCollapsed}
                  hiddenPinnedCount={pinning.hiddenCount}
                  hidePinnedToggle
                  onReorderPinned={pinning.onReorder}
                  virtualized
                  rowHeight={DEFAULT_TIMELINE_HEIGHT}
                />
              ) : undefined
            }
            pinnedHeader={
              hasPinnedSection ? (
                <PinnedSectionToggle
                  pinnedCount={pinning.items.length}
                  hiddenCount={pinning.hiddenCount}
                  isCollapsed={pinning.isCollapsed}
                  onToggle={pinning.onToggleCollapsed}
                />
              ) : undefined
            }
            pinnedCount={pinning?.items.length ?? 0}
            isCollapsed={pinning?.isCollapsed ?? false}
          >
            <TreeTable<TimelineTreeItem>
              data={data}
              columns={bodyColumns}
              initialSelectedItemId={controls.initialSelectedItemId}
              columnWidths={columnWidths}
              onColumnWidthsChange={setColumnWidths}
              onExpandChange={controls.onExpandChange}
              highlightedItemIds={controls.highlightedItemIds}
              indicatorItemIds={controls.indicatorItemIds}
              controlledExpandedIds={controls.expandedIds}
              virtualized
              rowHeight={DEFAULT_TIMELINE_HEIGHT}
            />
          </PinnedRowsSplit>
        ) : (
          <EmptyState
            size="compact"
            dimmed
            title={controls.emptyMessage ?? 'No timeline data available.'}
          />
        )}
      </div>
      {children}
    </div>
  );
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import type { TreeTableItem } from '@quent/components';
import type { TimelineTreeItem } from '../TimelineTreeTable';

export interface ResourceTimelineSubRow {
  id: string;
  /** Short name of the row, as plain text. */
  label: string;
  injectRows: (rootItem: TreeTableItem) => TreeTableItem;
  matches: (item: TimelineTreeItem) => boolean;
  renderLabel: (item: TimelineTreeItem, description?: ReactNode) => ReactNode;
  renderTimeline: (item: TimelineTreeItem) => ReactNode;
}

export function TimelineSubRowLabel({
  children,
  description,
}: {
  children: string;
  description?: ReactNode;
}) {
  return (
    <span className="flex items-center">
      <span aria-hidden className="mr-4 h-4 w-4 shrink-0" />
      <span className="flex flex-col gap-1">
        <span className="text-xs leading-none text-muted-foreground">{children}</span>
        {description != null && (
          <span className="text-[11px] font-normal leading-none text-muted-foreground">
            {description}
          </span>
        )}
      </span>
    </span>
  );
}

export function createSyntheticSubRow(id: string, type: string): TreeTableItem {
  return { id, type, entity: {} as TreeTableItem['entity'] };
}

/** Recursively rewrite each node's children; used to splice synthetic subrows into the tree. */
export function mapTreeItems(
  item: TreeTableItem,
  mapChildren: (item: TreeTableItem, children: TreeTableItem[]) => TreeTableItem[]
): TreeTableItem {
  const transformed = item.children?.map(child => mapTreeItems(child, mapChildren)) ?? [];
  const children = mapChildren(item, transformed);
  return children.length ? { ...item, children } : { ...item };
}

export function createTimelineSubRow({
  id,
  rowType,
  injectRows,
  label,
  renderTimeline,
}: {
  id: string;
  rowType: string;
  injectRows: ResourceTimelineSubRow['injectRows'];
  label: string;
  renderTimeline: ResourceTimelineSubRow['renderTimeline'];
}): ResourceTimelineSubRow {
  return {
    id,
    label,
    injectRows,
    matches: item => item.type === rowType,
    renderLabel: (_item, description) => (
      <TimelineSubRowLabel description={description}>{label}</TimelineSubRowLabel>
    ),
    renderTimeline,
  };
}

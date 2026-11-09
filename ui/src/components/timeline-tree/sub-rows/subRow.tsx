// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import type { TreeTableItem } from '@quent/components';
import type { TimelineTreeItem } from '../TimelineTreeTable';

export const TIMELINE_SUB_ROW_DISCLOSURE_HEIGHT = 28;

export interface ResourceTimelineSubRow {
  id: string;
  /** Short name of the row, as plain text. */
  label: string;
  /** Row type for the always-visible disclosure that owns the chart row. */
  disclosureRowType: string;
  /** Whether adding this disclosure turns a former leaf into an initially expanded owner. */
  expandOwnerByDefault: boolean;
  /**
   * Whether the chart row is pinned together with its owner instead of on its own. The owner's
   * pinned copy shows the chart row beneath it, and the chart row has no pin button.
   */
  pinsWithOwner: boolean;
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
    <span className="flex min-w-0 items-center">
      <span aria-hidden className="mr-4 h-4 w-4 shrink-0" />
      <span className="flex min-w-0 flex-col gap-1">
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

export function createTimelineSubRowDisclosure(
  ownerId: string,
  rowType: string,
  chartRow: TreeTableItem
): TreeTableItem {
  const idType = rowType.replace(/-/g, '_');
  return {
    ...createSyntheticSubRow(`__${idType}_disclosure__${ownerId}`, `${rowType}-disclosure`),
    children: [chartRow],
  };
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
  expandOwnerByDefault = false,
  pinsWithOwner = false,
}: {
  id: string;
  rowType: string;
  injectRows: ResourceTimelineSubRow['injectRows'];
  label: string;
  renderTimeline: ResourceTimelineSubRow['renderTimeline'];
  expandOwnerByDefault?: boolean;
  pinsWithOwner?: boolean;
}): ResourceTimelineSubRow {
  const disclosureRowType = `${rowType}-disclosure`;
  return {
    id,
    label,
    disclosureRowType,
    expandOwnerByDefault,
    pinsWithOwner,
    injectRows,
    matches: item => item.type === rowType || item.type === disclosureRowType,
    renderLabel: (item, description) =>
      item.type === disclosureRowType || description != null ? (
        <TimelineSubRowLabel description={description}>{label}</TimelineSubRowLabel>
      ) : null,
    renderTimeline: item => (item.type === disclosureRowType ? null : renderTimeline(item)),
  };
}

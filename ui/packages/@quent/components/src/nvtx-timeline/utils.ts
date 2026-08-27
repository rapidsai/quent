// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  DynamicAttribute,
  NvtxCatalog,
  NvtxCatalogDomain,
  NvtxCatalogThread,
  NvtxLane,
  NvtxLaneIdentity,
  NvtxMarkItem,
  NvtxRangeItem,
  NvtxViewportResponse,
} from '@quent/utils';
import { formatDuration } from '@quent/utils';
import type { TreeTableItem } from '../resource-tree/types';
import type { ActiveMark, TooltipItemNoun } from '../timeline/TimelineTooltip';

export const NVTX_SECTION_ROW_TYPE = 'nvtx-section';
export const NVTX_DOMAIN_ROW_TYPE = 'nvtx-domain';
export const NVTX_LANE_ROW_TYPE = 'nvtx-lane';

export const NVTX_SECTION_ID = '__nvtx__';
const DOMAIN_PREFIX = '__nvtx_domain__';
const THREAD_PREFIX = '__nvtx_thread__';
const PROCESS_PREFIX = '__nvtx_process__';
const MARKS_PREFIX = '__nvtx_marks__';

const RANGE_ITEM_NOUN: TooltipItemNoun = { singular: 'range', plural: 'ranges' };
const MARK_ITEM_NOUN: TooltipItemNoun = { singular: 'mark', plural: 'marks' };
const MIXED_ITEM_NOUN: TooltipItemNoun = { singular: 'item', plural: 'items' };

export type NvtxTreeEntity =
  | { nvtxKind: 'section' }
  | { nvtxKind: 'domain'; domain: NvtxCatalogDomain }
  | { nvtxKind: 'thread'; domain: NvtxCatalogDomain; thread: NvtxCatalogThread }
  | { nvtxKind: 'process'; domain: NvtxCatalogDomain }
  | { nvtxKind: 'marks'; domain: NvtxCatalogDomain };

export type NvtxTreeItem = TreeTableItem<NvtxTreeEntity>;

export function nvtxDomainRowId(domainId: string): string {
  return `${DOMAIN_PREFIX}${domainId}`;
}

export function nvtxThreadRowId(domainId: string, threadId: number): string {
  return `${THREAD_PREFIX}${domainId}__${threadId}`;
}

export function nvtxProcessRowId(domainId: string): string {
  return `${PROCESS_PREFIX}${domainId}`;
}

export function nvtxMarksRowId(domainId: string): string {
  return `${MARKS_PREFIX}${domainId}`;
}

export function isThreadIdentity(
  identity: NvtxLaneIdentity
): identity is Extract<NvtxLaneIdentity, { kind: 'thread' }> {
  return identity.kind === 'thread';
}

function treeItem(
  id: string,
  type: string,
  entity: NvtxTreeEntity,
  children?: NvtxTreeItem[]
): NvtxTreeItem {
  return { id, type, entity, ...(children?.length ? { children } : {}) };
}

/** Domain sub-trees for the visible domains; headers stay so category filters have a row. */
export function buildNvtxTree(
  catalog: Pick<NvtxCatalog, 'domains'>,
  laneRowIds: ReadonlySet<string>,
  selectedDomainId: string | null = null
): NvtxTreeItem | null {
  const visibleDomains = catalog.domains.filter(
    domain => selectedDomainId == null || domain.domain_id === selectedDomainId
  );
  if (visibleDomains.length === 0) {
    return null;
  }
  const domainLanes = visibleDomains.map(domain => {
    const threadRows = domain.threads.map(thread =>
      treeItem(nvtxThreadRowId(domain.domain_id, thread.thread_id), NVTX_LANE_ROW_TYPE, {
        nvtxKind: 'thread',
        domain,
        thread,
      })
    );
    const extraRows: NvtxTreeItem[] = [];
    if (laneRowIds.has(nvtxProcessRowId(domain.domain_id))) {
      extraRows.push(
        treeItem(nvtxProcessRowId(domain.domain_id), NVTX_LANE_ROW_TYPE, {
          nvtxKind: 'process',
          domain,
        })
      );
    }
    if (laneRowIds.has(nvtxMarksRowId(domain.domain_id))) {
      extraRows.push(
        treeItem(nvtxMarksRowId(domain.domain_id), NVTX_LANE_ROW_TYPE, {
          nvtxKind: 'marks',
          domain,
        })
      );
    }
    return [...threadRows, ...extraRows];
  });
  const children = visibleDomains.map((domain, index) =>
    treeItem(
      nvtxDomainRowId(domain.domain_id),
      NVTX_DOMAIN_ROW_TYPE,
      { nvtxKind: 'domain', domain },
      domainLanes[index]
    )
  );
  return treeItem(NVTX_SECTION_ID, NVTX_SECTION_ROW_TYPE, { nvtxKind: 'section' }, children);
}

/** Map tree row id → viewport lanes (thread depths grouped, process/marks as one lane). */
export function indexNvtxLanes(viewport: NvtxViewportResponse | null): Map<string, NvtxLane[]> {
  const lanesByRowId = new Map<string, NvtxLane[]>();
  if (!viewport) {
    return lanesByRowId;
  }
  for (const domain of viewport.domains) {
    const byThread = new Map<number, NvtxLane[]>();
    for (const lane of domain.lanes) {
      if (isThreadIdentity(lane.identity)) {
        const lanes = byThread.get(lane.identity.thread_id) ?? [];
        lanes.push(lane);
        byThread.set(lane.identity.thread_id, lanes);
      } else if (lane.identity.kind === 'process') {
        lanesByRowId.set(nvtxProcessRowId(domain.domain_id), [lane]);
      } else if (lane.identity.kind === 'marks') {
        lanesByRowId.set(nvtxMarksRowId(domain.domain_id), [lane]);
      }
    }
    for (const [threadId, lanes] of byThread) {
      lanes.sort((left, right) => {
        const leftDepth = isThreadIdentity(left.identity) ? left.identity.depth : 0;
        const rightDepth = isThreadIdentity(right.identity) ? right.identity.depth : 0;
        return leftDepth - rightDepth;
      });
      lanesByRowId.set(nvtxThreadRowId(domain.domain_id, threadId), lanes);
    }
  }
  return lanesByRowId;
}

export function isNvtxTreeEntity(entity: unknown): entity is NvtxTreeEntity {
  if (typeof entity !== 'object' || entity === null || !('nvtxKind' in entity)) {
    return false;
  }
  return ['section', 'domain', 'thread', 'process', 'marks'].includes(String(entity.nvtxKind));
}

export function nvtxDomainMeta(entity: NvtxTreeEntity): { name: string; color: string } | null {
  if (entity.nvtxKind !== 'domain') {
    return null;
  }
  return { name: entity.domain.name, color: rgbHex(entity.domain.color) };
}

export function nvtxLaneLabel(entity: NvtxTreeEntity, includeDomain = false): string {
  if (entity.nvtxKind === 'section' || entity.nvtxKind === 'domain') {
    return '';
  }
  const prefix = includeDomain ? `${entity.domain.name} · ` : '';
  if (entity.nvtxKind === 'process') {
    return `${prefix}Process ranges`;
  }
  if (entity.nvtxKind === 'marks') {
    return `${prefix}Marks`;
  }
  return `${prefix}${entity.thread.name}`;
}

export type NvtxGanttDatum = {
  value: [number, number, number];
  range?: NvtxRangeItem;
  mark?: NvtxMarkItem;
  /** Set when adjacent same-color bars collapsed to one pixel column. */
  mergedCount?: number;
  /** Per-message counts retained for a merged block's tooltip. */
  mergedTypeCounts?: Array<{ label: string; count: number }>;
};

export interface NvtxPixelBudget {
  visibleStartMs: number;
  visibleEndMs: number;
  plotWidthPx: number;
}

export const NVTX_MIN_BAR_WIDTH_PX = 2;

/** Flatten populated viewport lanes into contiguous Gantt rows. */
export function nvtxLanesToGanttData(lanes: NvtxLane[]): NvtxGanttDatum[] {
  const data: NvtxGanttDatum[] = [];
  const populatedLanes = lanes.filter(lane => lane.ranges.length > 0 || lane.marks.length > 0);
  for (const [rowIndex, lane] of populatedLanes.entries()) {
    for (const range of lane.ranges) {
      data.push({
        value: [range.display_start * 1_000, range.display_end * 1_000, rowIndex],
        range,
      });
    }
    for (const mark of lane.marks) {
      const timestampMs = mark.timestamp * 1_000;
      data.push({
        value: [timestampMs, timestampMs, rowIndex],
        mark,
      });
    }
  }
  return data;
}

/** Merge same-row, same-color bars whose pixel occupancy is this close. */
export const NVTX_BAR_MERGE_GAP_PX = 2;
/** Minimum touching bars required before collapsing them into one. */
export const NVTX_BAR_MERGE_MIN_COUNT = 8;

/** Collapse same-row, same-color bars that would occupy the same pixels. */
export function mergeNvtxGanttData(
  data: NvtxGanttDatum[],
  budget: NvtxPixelBudget
): NvtxGanttDatum[] {
  const spanMs = budget.visibleEndMs - budget.visibleStartMs;
  if (data.length <= 1 || budget.plotWidthPx <= 0 || spanMs <= 0) {
    return data;
  }
  const msPerPx = spanMs / budget.plotWidthPx;
  const groups = new Map<string, NvtxGanttDatum[]>();
  for (const datum of data) {
    const key = nvtxBarMergeKey(datum);
    const group = groups.get(key);
    if (group) {
      group.push(datum);
    } else {
      groups.set(key, [datum]);
    }
  }
  const merged: NvtxGanttDatum[] = [];
  for (const group of groups.values()) {
    merged.push(...mergeNvtxBarGroup(group, budget.visibleStartMs, msPerPx));
  }
  return merged;
}

function nvtxBarMergeKey(datum: NvtxGanttDatum): string {
  const color = rgbHex(datum.range?.color ?? datum.mark?.color ?? '');
  const kind = datum.mark ? 'm' : 'r';
  return `${datum.value[2]}:${kind}:${color}`;
}

function mergeNvtxBarGroup(
  group: NvtxGanttDatum[],
  originMs: number,
  msPerPx: number
): NvtxGanttDatum[] {
  if (group.length <= 1) {
    return group;
  }
  const sorted = [...group].sort((left, right) => left.value[0] - right.value[0]);
  const out: NvtxGanttDatum[] = [];
  let run = [sorted[0]!];
  let startMs = run[0]!.value[0];
  let endMs = run[0]!.value[1];
  let endPx = barEndPx(startMs, endMs, originMs, msPerPx);
  for (let index = 1; index < sorted.length; index++) {
    const next = sorted[index]!;
    const nextStartPx = (next.value[0] - originMs) / msPerPx;
    if (nextStartPx <= endPx + NVTX_BAR_MERGE_GAP_PX) {
      run.push(next);
      endMs = Math.max(endMs, next.value[1]);
      endPx = Math.max(endPx, barEndPx(next.value[0], next.value[1], originMs, msPerPx));
      continue;
    }
    out.push(...condenseNvtxBarRun(run, startMs, endMs));
    run = [next];
    startMs = next.value[0];
    endMs = next.value[1];
    endPx = barEndPx(startMs, endMs, originMs, msPerPx);
  }
  out.push(...condenseNvtxBarRun(run, startMs, endMs));
  return out;
}

function condenseNvtxBarRun(
  run: NvtxGanttDatum[],
  startMs: number,
  endMs: number
): NvtxGanttDatum[] {
  if (run.length < NVTX_BAR_MERGE_MIN_COUNT) {
    return run;
  }
  return [nvtxMergedDatum(run, startMs, endMs)];
}

function barEndPx(startMs: number, endMs: number, originMs: number, msPerPx: number): number {
  const startPx = (startMs - originMs) / msPerPx;
  return Math.max((Math.max(endMs, startMs) - originMs) / msPerPx, startPx + 1);
}

function nvtxMergedDatum(run: NvtxGanttDatum[], startMs: number, endMs: number): NvtxGanttDatum {
  const datum = run[0]!;
  const counts = new Map<string, number>();
  for (const item of run) {
    const label = item.range?.message ?? item.mark?.message;
    if (label) {
      counts.set(label, (counts.get(label) ?? 0) + 1);
    }
  }
  return {
    ...datum,
    value: [startMs, endMs, datum.value[2]],
    mergedCount: run.length,
    mergedTypeCounts: [...counts].map(([label, count]) => ({ label, count })),
  };
}

export function nvtxItemsAtTimestamp(
  data: NvtxGanttDatum[],
  timestampMs: number,
  minimumHitWidthMs: number
): NvtxGanttDatum[] {
  return data.filter(datum => {
    const [startMs, endMs] = datum.value;
    const hitEnd = Math.max(endMs, startMs + minimumHitWidthMs);
    return startMs <= timestampMs && timestampMs < hitEnd;
  });
}

function stringAttr(key: string, value: string): DynamicAttribute {
  return { key, value };
}

function countLabel(count: number, singular: string, plural: string): string {
  return count === 1 ? `1 ${singular}` : `${count} ${plural}`;
}

/** Compact name + count row for pixel-merged bars. */
export function nvtxToSummaryMark(datum: NvtxGanttDatum): ActiveMark {
  const count = datum.mergedCount ?? 1;
  if (datum.mark) {
    return {
      label: 'Consolidated block',
      stateName: countLabel(count, 'mark', 'marks'),
      color: rgbHex(datum.mark.color),
      compact: true,
    };
  }
  const range = datum.range!;
  return {
    label: 'Consolidated block',
    stateName: countLabel(count, 'range', 'ranges'),
    color: rgbHex(range.color),
    compact: true,
  };
}

function nvtxToSummaryMarks(datum: NvtxGanttDatum): ActiveMark[] {
  const typeCounts = datum.mergedTypeCounts;
  if (!typeCounts?.length) {
    return [nvtxToSummaryMark(datum)];
  }
  const isMark = datum.mark != null;
  const color = rgbHex(datum.mark?.color ?? datum.range?.color ?? '');
  return typeCounts.map(({ label, count }) => ({
    label,
    stateName: countLabel(count, isMark ? 'mark' : 'range', isMark ? 'marks' : 'ranges'),
    color,
    compact: true,
  }));
}

export const NVTX_TOOLTIP_COMPACT_LIMIT = 6;
export const NVTX_TOOLTIP_DETAIL_LIMIT = 3;

export type NvtxTooltipModel = {
  marks: ActiveMark[];
  summary?: string;
  compact: boolean;
  itemLimit: number;
  itemNoun: TooltipItemNoun;
};

/** Count rows for merged bars; full range data when the item is a single range. */
export function nvtxTooltipModel(data: NvtxGanttDatum[]): NvtxTooltipModel {
  const orderedData = [...data].sort((left, right) => left.value[2] - right.value[2]);
  const hasMerged = orderedData.some(datum => (datum.mergedCount ?? 1) > 1);
  const hasSingle = orderedData.some(datum => (datum.mergedCount ?? 1) === 1);
  const rangeCount = orderedData.reduce(
    (sum, datum) => sum + (datum.range ? (datum.mergedCount ?? 1) : 0),
    0
  );
  const markCount = orderedData.reduce(
    (sum, datum) => sum + (datum.mark ? (datum.mergedCount ?? 1) : 0),
    0
  );
  const parts = [
    ...(rangeCount > 0 ? [countLabel(rangeCount, 'range', 'ranges')] : []),
    ...(markCount > 0 ? [countLabel(markCount, 'mark', 'marks')] : []),
  ];
  const itemNoun =
    rangeCount > 0 && markCount === 0
      ? RANGE_ITEM_NOUN
      : markCount > 0 && rangeCount === 0
        ? MARK_ITEM_NOUN
        : MIXED_ITEM_NOUN;
  return {
    marks: orderedData.flatMap(datum =>
      (datum.mergedCount ?? 1) > 1 ? nvtxToSummaryMarks(datum) : [nvtxToActiveMark(datum)]
    ),
    summary: hasMerged ? parts.join(', ') : undefined,
    compact: hasMerged,
    itemLimit: hasSingle ? NVTX_TOOLTIP_DETAIL_LIMIT : NVTX_TOOLTIP_COMPACT_LIMIT,
    itemNoun,
  };
}

/** Map a Gantt datum onto the shared TimelineTooltip mark shape. */
export function nvtxToActiveMark(datum: NvtxGanttDatum): ActiveMark {
  const mergedCount = datum.mergedCount ?? 1;
  if (datum.mark) {
    if (mergedCount > 1) {
      return nvtxToSummaryMark(datum);
    }
    return {
      label: datum.mark.domain_name,
      stateName: datum.mark.message,
      color: rgbHex(datum.mark.color),
      attributes: [
        stringAttr('kind', nvtxKindLabel('mark')),
        stringAttr('category', datum.mark.category_name ?? 'Uncategorized'),
      ],
    };
  }
  const range = datum.range!;
  if (mergedCount > 1) {
    return nvtxToSummaryMark(datum);
  }
  const threadAttributes = [
    ...(range.thread_name != null ? [stringAttr('thread', range.thread_name)] : []),
    ...(range.thread_id != null ? [stringAttr('thread ID', range.thread_id.toString())] : []),
  ];
  return {
    label: range.message,
    stateName: '',
    color: rgbHex(range.color),
    durationMs: range.observed_duration != null ? range.observed_duration * 1_000 : undefined,
    attributes: [
      stringAttr('start', formatDuration(range.observed_start * 1_000)),
      stringAttr(
        'end',
        range.observed_end != null ? formatDuration(range.observed_end * 1_000) : '(open)'
      ),
      stringAttr('kind', nvtxKindLabel(range.kind)),
      stringAttr('domain', range.domain_name),
      stringAttr('category', range.category_name ?? 'Uncategorized'),
      ...threadAttributes,
    ],
  };
}

export function rgbHex(color: string): string {
  return color.length >= 7 ? color.slice(0, 7) : color;
}

const MERGED_COUNT_CHARACTER_WIDTH = 5;
const MERGED_COUNT_PADDING = 4;
const MERGED_COUNT_OPACITY = 0.6;

/** Shows the exact item count when a merged bar is wide enough to read it. */
export function nvtxMergedBarCountLabel(
  shape: { x: number; y: number; width: number; height: number },
  fill: string,
  count: number,
  kind: 'mark' | 'range'
): Array<{ type: 'text'; silent: true; style: object }> {
  const text = `(${count} ${count === 1 ? kind : `${kind}s`})`;
  if (shape.width < text.length * MERGED_COUNT_CHARACTER_WIDTH + MERGED_COUNT_PADDING) {
    return [];
  }
  const cy = shape.y + shape.height / 2;
  return [
    {
      type: 'text',
      silent: true,
      style: {
        text,
        x: shape.x + shape.width / 2,
        y: cy,
        textAlign: 'center',
        textVerticalAlign: 'middle',
        fontSize: 9,
        fill,
        opacity: MERGED_COUNT_OPACITY,
      },
    },
  ];
}

export function nvtxKindLabel(kind: NvtxRangeItem['kind'] | 'mark'): string {
  if (kind === 'mark') {
    return 'mark';
  }
  if (kind === 'push_pop') {
    return 'push/pop range';
  }
  return 'start/end range';
}

export function nvtxDefaultExpandedIds(catalog: NvtxCatalog): string[] {
  return [NVTX_SECTION_ID, ...catalog.domains.map(domain => nvtxDomainRowId(domain.domain_id))];
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod';
import {
  CONTINUOUS_PALETTES,
  DAG_LAYOUT_DIRECTION,
  NODE_LABEL_FIELD,
  AGG_MODES,
} from '@quent/utils';
import { OPERATOR_TABLE_INDEX_ORDER } from '@/components/operator-table/types';
import { MAX_RESOURCE_FILTER_QUERY_LENGTH } from '@/features/resource-filter/resourceFilter';

export const MAX_ENCODED_STATE_LENGTH = 4096;
export const MAX_EXPANDED_RESOURCE_IDS = 50;
export const MAX_SELECTED_NODE_IDS = 50;
export const MAX_RESOURCE_OVERRIDES = 50;
export const MAX_VISIBLE_STATS = 100;
export const MAX_TABLE_SORTS = 5;

const MAX_V1_EXPANDED_RESOURCE_IDS = 100;
const MAX_ID_LENGTH = 128;
const MAX_NAME_LENGTH = 256;

const IdSchema = z.string().min(1).max(MAX_ID_LENGTH);
const NameSchema = z.string().min(1).max(MAX_NAME_LENGTH);

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values)].sort();
}

function uniqueInOrder<T>(values: T[]): T[] {
  return [...new Set(values)];
}

function canonicalSelections<T extends { rowId: string }>(values: T[]): T[] {
  return [...new Map(values.map(value => [value.rowId, value])).values()].sort((a, b) =>
    a.rowId.localeCompare(b.rowId)
  );
}

function enumKeys<T extends Record<string, unknown>>(
  obj: T
): [keyof T & string, ...(keyof T & string)[]] {
  return Object.keys(obj) as [keyof T & string, ...(keyof T & string)[]];
}

export const ZoomRangeSchema = z
  .object({
    start: z.number().finite().nonnegative(),
    end: z.number().finite().positive(),
  })
  .refine(range => range.end > range.start, {
    message: 'Zoom range end must be greater than start',
  });

export const DeepLinkStateV1Schema = z
  .object({
    zoomRange: ZoomRangeSchema,
    expandedResourceIds: z
      .array(z.string().min(1).max(1024))
      .max(MAX_V1_EXPANDED_RESOURCE_IDS)
      .transform(ids => [...new Set(ids)].sort())
      .optional(),
  })
  .strip();

export type DeepLinkStateV1 = z.infer<typeof DeepLinkStateV1Schema>;

const RouteSchema = z
  .object({
    engineId: IdSchema,
    queryId: IdSchema,
    tab: z.enum(['timeline', 'operators']),
  })
  .strip();

const SelectionSchema = z
  .object({
    planId: IdSchema.optional(),
    operatorNodeIds: z
      .array(IdSchema)
      .max(MAX_SELECTED_NODE_IDS)
      .transform(uniqueSorted)
      .optional(),
  })
  .strip();

const ResourceSelectionSchema = z
  .object({
    rowId: IdSchema,
    resourceType: NameSchema,
  })
  .strip();

const FsmSelectionSchema = z
  .object({
    rowId: IdSchema,
    fsmType: NameSchema.nullable(),
  })
  .strip();

const ResourceFilterSchema = z
  .object({
    search: z.string().trim().min(1).max(MAX_RESOURCE_FILTER_QUERY_LENGTH).optional(),
    resourceTypes: z
      .array(NameSchema)
      .max(MAX_RESOURCE_OVERRIDES)
      .transform(uniqueSorted)
      .optional(),
    fsmTypes: z.array(NameSchema).max(MAX_RESOURCE_OVERRIDES).transform(uniqueSorted).optional(),
    showOthers: z.boolean().optional(),
  })
  .strip();

const ResourceTreeSchema = z
  .object({
    expandedRowIds: z
      .array(IdSchema)
      .max(MAX_EXPANDED_RESOURCE_IDS)
      .transform(uniqueSorted)
      .optional(),
    rootResourceType: NameSchema.optional(),
    resourceFilter: ResourceFilterSchema.optional(),
    resourceTypeSelections: z
      .array(ResourceSelectionSchema)
      .max(MAX_RESOURCE_OVERRIDES)
      .transform(canonicalSelections)
      .optional(),
    fsmSelections: z
      .array(FsmSelectionSchema)
      .max(MAX_RESOURCE_OVERRIDES)
      .transform(canonicalSelections)
      .optional(),
  })
  .strip();

const ContinuousPaletteSchema = z.enum(enumKeys(CONTINUOUS_PALETTES));

const DagControlsSchema = z
  .object({
    nodeColorField: NameSchema.optional(),
    nodeColorPalette: ContinuousPaletteSchema.optional(),
    edgeWidthField: NameSchema.optional(),
    edgeColorField: NameSchema.optional(),
    edgeColorPalette: ContinuousPaletteSchema.optional(),
    nodeLabelField: z.enum(NODE_LABEL_FIELD).optional(),
    layoutDirection: z.enum(DAG_LAYOUT_DIRECTION).optional(),
  })
  .strip();

const DataFlowSchema = z
  .object({
    enabled: z.boolean().optional(),
    measure: NameSchema.optional(),
    labelMeasure: NameSchema.optional(),
    dimensions: z.array(NameSchema).min(1).max(32).transform(uniqueSorted).optional(),
    playheadS: z.number().finite().nonnegative().optional(),
  })
  .strip();

export const OperatorGroupSchema = z.enum(OPERATOR_TABLE_INDEX_ORDER);

const OperatorTableSchema = z
  .object({
    groupingOrder: z
      .array(OperatorGroupSchema)
      .max(OperatorGroupSchema.options.length)
      .transform(uniqueInOrder)
      .optional(),
    enabledGroups: z
      .array(OperatorGroupSchema)
      .max(OperatorGroupSchema.options.length)
      .transform(uniqueInOrder)
      .optional(),
    visibleStats: z.array(NameSchema).max(MAX_VISIBLE_STATS).transform(uniqueInOrder).optional(),
    aggregation: z.enum(AGG_MODES).optional(),
    sort: z
      .array(
        z
          .object({
            id: NameSchema,
            desc: z.boolean(),
          })
          .strip()
      )
      .max(MAX_TABLE_SORTS)
      .optional(),
  })
  .strip();

export const DeepLinkStateV2Schema = z
  .object({
    route: RouteSchema,
    timeline: z.object({ zoomRange: ZoomRangeSchema }).strip(),
    selection: SelectionSchema.optional(),
    resources: ResourceTreeSchema.optional(),
    dag: DagControlsSchema.optional(),
    dataFlow: DataFlowSchema.optional(),
    operatorTable: OperatorTableSchema.optional(),
  })
  .strip();

export type DeepLinkStateV2 = z.infer<typeof DeepLinkStateV2Schema>;
export type DeepLinkState = DeepLinkStateV2;
export type DeepLinkTab = DeepLinkStateV2['route']['tab'];

export const DeepLinkSearchSchema = z
  .object({
    s: z.string().optional(),
  })
  .strip();

export type DeepLinkSearch = z.infer<typeof DeepLinkSearchSchema>;

export function validateDeepLinkSearch(search: unknown): DeepLinkSearch {
  const result = DeepLinkSearchSchema.safeParse(search);
  return result.success ? result.data : {};
}

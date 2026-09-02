// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod';
import {
  CONTINUOUS_PALETTES,
  DAG_LAYOUT_DIRECTION,
  NODE_LABEL_FIELD,
  AGG_MODES,
} from '@quent/utils';
import { MAX_PAGE_SIZE } from '@/components/entities-table/utils';
import { OPERATOR_TABLE_INDEX_ORDER } from '@/components/operator-table/types';

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

const RouteV3Schema = z
  .object({
    engineId: IdSchema,
    queryId: IdSchema,
    tab: z.enum(['timeline', 'operators', 'entities']),
  })
  .strip();

const TimelineSchema = z.object({ zoomRange: ZoomRangeSchema }).strip();

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

const ResourceTreeSchema = z
  .object({
    expandedRowIds: z
      .array(IdSchema)
      .max(MAX_EXPANDED_RESOURCE_IDS)
      .transform(uniqueSorted)
      .optional(),
    rootResourceType: NameSchema.optional(),
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

const EntityWindowSchema = z
  .object({
    start: z.number().finite().nonnegative(),
    end: z.number().finite().nonnegative(),
  })
  .refine(window => window.end >= window.start, {
    message: 'Entity window end must not precede its start',
  });

const EntitiesSchema = z
  .object({
    operatorId: IdSchema.nullable().optional(),
    entityType: NameSchema.optional(),
    resourceId: IdSchema.optional(),
    minUsageS: z.number().finite().nonnegative().optional(),
    window: EntityWindowSchema.optional(),
    sortDir: z.enum(['Asc', 'Desc']).optional(),
    pageSize: z.number().int().min(1).max(MAX_PAGE_SIZE).optional(),
    page: z.number().int().nonnegative().max(1_000_000).optional(),
    selectedEntityId: IdSchema.optional(),
  })
  .strip();

export const DeepLinkStateV2Schema = z
  .object({
    route: RouteSchema,
    timeline: TimelineSchema,
    selection: SelectionSchema.optional(),
    resources: ResourceTreeSchema.optional(),
    dag: DagControlsSchema.optional(),
    dataFlow: DataFlowSchema.optional(),
    operatorTable: OperatorTableSchema.optional(),
  })
  .strip();

export type DeepLinkStateV2 = z.infer<typeof DeepLinkStateV2Schema>;

export const DeepLinkStateV3Schema = z
  .object({
    route: RouteV3Schema,
    timeline: TimelineSchema.optional(),
    selection: SelectionSchema.optional(),
    resources: ResourceTreeSchema.optional(),
    dag: DagControlsSchema.optional(),
    dataFlow: DataFlowSchema.optional(),
    operatorTable: OperatorTableSchema.optional(),
    entities: EntitiesSchema.optional(),
  })
  .strip()
  .superRefine((state, context) => {
    if (state.route.tab !== 'entities' && !state.timeline) {
      context.addIssue({
        code: 'custom',
        path: ['timeline'],
        message: 'Timeline state is required for timeline and operator links',
      });
    }
    if (state.route.tab !== 'entities' && state.entities) {
      context.addIssue({
        code: 'custom',
        path: ['entities'],
        message: 'Entity state is only valid for entity links',
      });
    }
  });

export type DeepLinkStateV3 = z.infer<typeof DeepLinkStateV3Schema>;
export type DeepLinkState = DeepLinkStateV3;
export type DeepLinkTab = DeepLinkStateV3['route']['tab'];
export type DeepLinkEntitiesState = NonNullable<DeepLinkStateV3['entities']>;

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

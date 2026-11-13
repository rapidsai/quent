// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod';
import { MAX_ENCODED_STATE_LENGTH } from '@/features/deep-link/deepLink.schema';

export const SAVED_VIEWS_SCHEMA = 'quent.saved-views';
export const SAVED_VIEWS_VERSION = 1;
export const MAX_SAVED_VIEWS_PER_QUERY = 50;
export const MAX_SAVED_VIEW_NAME_LENGTH = 80;
export const MAX_SAVED_VIEW_NVTX_CATEGORIES = 50;

const ScopedIdSchema = z.string().min(1).max(128);
const SavedViewNameSchema = z.string().trim().min(1).max(MAX_SAVED_VIEW_NAME_LENGTH);

export const SavedViewExtrasV1Schema = z
  .object({
    pinnedCollapsed: z.literal(true).optional(),
    // LONG_ENTITY_DENSITY_DEFAULT is omitted so default-valued extras stay compact.
    longEntityDensity: z.union([z.literal(1), z.literal(2), z.literal(3), z.literal(5)]).optional(),
    nvtx: z
      .object({
        domainId: ScopedIdSchema.optional(),
        categories: z
          .array(
            z
              .object({
                domainId: ScopedIdSchema,
                categoryId: ScopedIdSchema,
              })
              .strip()
          )
          .max(MAX_SAVED_VIEW_NVTX_CATEGORIES)
          .optional(),
      })
      .strip()
      .optional(),
  })
  .strip();

export const SavedViewV1Schema = z
  .object({
    id: z.uuid(),
    name: SavedViewNameSchema,
    createdAt: z.number().int().nonnegative(),
    updatedAt: z.number().int().nonnegative(),
    tab: z.enum(['timeline', 'operators', 'entities']),
    payload: z
      .string()
      .min(1)
      .max(MAX_ENCODED_STATE_LENGTH)
      .regex(/^v\d+\.[A-Za-z0-9_-]+$/u),
    extras: SavedViewExtrasV1Schema.optional(),
  })
  .strip()
  .refine(view => view.updatedAt >= view.createdAt, {
    path: ['updatedAt'],
    message: 'Updated time cannot precede created time',
  });

export const SavedViewsDocumentHeaderV1Schema = z
  .object({
    schema: z.literal(SAVED_VIEWS_SCHEMA),
    version: z.number().int().nonnegative(),
    engineId: ScopedIdSchema,
    queryId: ScopedIdSchema,
    views: z.array(z.unknown()),
  })
  .passthrough();

export type SavedViewExtrasV1 = z.infer<typeof SavedViewExtrasV1Schema>;
export type SavedViewV1 = z.infer<typeof SavedViewV1Schema>;

export interface SavedViewsDocumentV1 {
  schema: typeof SAVED_VIEWS_SCHEMA;
  version: typeof SAVED_VIEWS_VERSION;
  engineId: string;
  queryId: string;
  views: unknown[];
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod';
import { MAX_ENCODED_STATE_LENGTH } from '@/features/deep-link/deepLink.schema';

export const VIEW_AUTOSAVE_SCHEMA = 'quent.view-autosave';
export const VIEW_AUTOSAVE_VERSION = 1;
export const MAX_NVTX_CATEGORIES = 50;

const ScopedIdSchema = z.string().min(1).max(128);

/** View state the shared-link payload does not carry. */
export const ViewExtrasSchema = z
  .object({
    pinnedCollapsed: z.literal(true).optional(),
    // The default density is omitted so untouched views stay compact.
    longEntityDensity: z.union([z.literal(1), z.literal(2), z.literal(3), z.literal(5)]).optional(),
    // Height in pixels of the pinned-rows panel after the user resized it.
    pinnedPanelSize: z.number().positive().max(10_000).optional(),
    nvtx: z
      .object({
        domainId: ScopedIdSchema.optional(),
        categories: z
          .array(z.object({ domainId: ScopedIdSchema, categoryId: ScopedIdSchema }).strip())
          .max(MAX_NVTX_CATEGORIES)
          .optional(),
      })
      .strip()
      .optional(),
  })
  .strip();

export const ViewAutosaveRecordSchema = z
  .object({
    schema: z.literal(VIEW_AUTOSAVE_SCHEMA),
    version: z.literal(VIEW_AUTOSAVE_VERSION),
    engineId: ScopedIdSchema,
    queryId: ScopedIdSchema,
    updatedAt: z.number().int().nonnegative(),
    tab: z.enum(['timeline', 'operators', 'entities']),
    payload: z
      .string()
      .min(1)
      .max(MAX_ENCODED_STATE_LENGTH)
      .regex(/^v\d+\.[A-Za-z0-9_-]+$/u),
    extras: ViewExtrasSchema.optional(),
  })
  .strip();

export type ViewExtras = z.infer<typeof ViewExtrasSchema>;
export type ViewAutosaveRecord = z.infer<typeof ViewAutosaveRecordSchema>;

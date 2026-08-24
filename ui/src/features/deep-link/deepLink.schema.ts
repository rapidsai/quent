// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { z } from 'zod';

export const MAX_ENCODED_STATE_LENGTH = 4096;
export const MAX_EXPANDED_RESOURCE_IDS = 100;

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
      .array(z.uuid())
      .max(MAX_EXPANDED_RESOURCE_IDS)
      .transform(ids => [...new Set(ids)].sort())
      .optional(),
  })
  .strip();

export type DeepLinkStateV1 = z.infer<typeof DeepLinkStateV1Schema>;

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

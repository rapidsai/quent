// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { compressToEncodedURIComponent, decompressFromEncodedURIComponent } from 'lz-string';
import { z } from 'zod';

const treeStateSchema = z.object({
  expandedIds: z.array(z.string()).catch([]),
  selectedTypes: z.record(z.string(), z.string()).catch({}),
  selectedFsmTypes: z.record(z.string(), z.string().nullable()).catch({}),
});

export type TreeState = z.infer<typeof treeStateSchema>;

export interface TreeStateInput {
  expandedIds: Set<string>;
  selectedTypes: Map<string, string>;
  selectedFsmTypes: Map<string, string | null>;
}

export function encodeTreeState(state: TreeStateInput): string {
  const raw = {
    expandedIds: [...state.expandedIds],
    selectedTypes: Object.fromEntries(state.selectedTypes),
    selectedFsmTypes: Object.fromEntries(state.selectedFsmTypes),
  };
  return compressToEncodedURIComponent(JSON.stringify(raw));
}

export function decodeTreeState(param: string): TreeState | null {
  try {
    const decompressed = decompressFromEncodedURIComponent(param);
    if (!decompressed) return null;
    const parsed: unknown = JSON.parse(decompressed);
    const result = treeStateSchema.safeParse(parsed);
    return result.success ? result.data : null;
  } catch {
    return null;
  }
}

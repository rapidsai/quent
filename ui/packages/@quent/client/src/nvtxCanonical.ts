// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { NvtxDomainSelection, NvtxViewportRequest } from '@quent/utils';

export function canonicalizeNvtxSelections(
  selections: readonly NvtxDomainSelection[]
): NvtxDomainSelection[] {
  const seen = new Set<string>();
  const canonical = selections.map(selection => {
    const domainKey = BigInt(selection.domain_id).toString(10);
    if (seen.has(domainKey)) throw new Error(`duplicate NVTX domain ${domainKey}`);
    seen.add(domainKey);
    const category_ids = [...new Set(selection.category_ids)].sort((left, right) => left - right);
    if (category_ids.length === 0 && !selection.include_uncategorized) {
      throw new Error(`NVTX domain ${domainKey} selects no categories`);
    }
    return { ...selection, domain_id: domainKey, category_ids };
  });
  canonical.sort((left, right) =>
    BigInt(left.domain_id) < BigInt(right.domain_id)
      ? -1
      : BigInt(left.domain_id) > BigInt(right.domain_id)
        ? 1
        : 0
  );
  return canonical;
}

export function canonicalizeNvtxRequest(request: NvtxViewportRequest): NvtxViewportRequest {
  const { start, end } = request.viewport;
  if (!Number.isFinite(start) || !Number.isFinite(end) || start > end) {
    throw new Error('NVTX viewport bounds must be finite and ordered');
  }
  return {
    viewport: { start, end },
    selections: canonicalizeNvtxSelections(request.selections),
  };
}

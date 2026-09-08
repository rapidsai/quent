// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { TimelineRequest, OperatorFilter } from '@quent/utils';
export { getFsmTypeName, getResourceTypeName } from '@quent/utils';

/** Canonicalize an operator filter for stable request and cache keys. */
export function canonicalOperatorIds(operatorIds?: readonly string[] | null): string[] {
  return [...new Set(operatorIds ?? [])].sort();
}

/** Stable request-entry key for bulk timeline fetches. Omit operatorIds for the base variant. */
export function bulkEntryId(resourceId: string, operatorIds?: readonly string[] | null): string {
  const canonicalIds = canonicalOperatorIds(operatorIds);
  return canonicalIds.length > 0
    ? `${resourceId}:ops:${JSON.stringify(canonicalIds)}`
    : `${resourceId}:base`;
}

/** Clone an entry and set its operator filter. */
export function setOperatorOnEntry(
  entry: TimelineRequest<OperatorFilter>,
  operatorIds: readonly string[]
): TimelineRequest<OperatorFilter> {
  if ('ResourceGroup' in entry) {
    return {
      ResourceGroup: {
        ...entry.ResourceGroup,
        app_params: { ...entry.ResourceGroup.app_params, operator_ids: [...operatorIds] },
      },
    };
  }
  return {
    Resource: {
      ...entry.Resource,
      application: { ...entry.Resource.application, operator_ids: [...operatorIds] },
    },
  };
}

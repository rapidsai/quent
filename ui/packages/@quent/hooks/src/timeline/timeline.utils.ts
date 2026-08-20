// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { TimelineRequest, OperatorFilter } from '@quent/utils';
export { getFsmTypeName, getResourceTypeName } from '@quent/utils';

/** Stable request-entry key for bulk timeline fetches. Omit operatorId for the base variant. */
export function bulkEntryId(resourceId: string, operatorId?: string | null): string {
  return operatorId ? `${resourceId}:op:${operatorId}` : `${resourceId}:base`;
}

/** Clone entries and set operator_id on each TimelineRequest */
export function setOperatorOnEntry(
  entry: TimelineRequest<OperatorFilter>,
  operatorId: string
): TimelineRequest<OperatorFilter> {
  if ('ResourceGroup' in entry) {
    return {
      ResourceGroup: {
        ...entry.ResourceGroup,
        app_params: { ...entry.ResourceGroup.app_params, operator_ids: [operatorId] },
      },
    };
  }
  return {
    Resource: {
      ...entry.Resource,
      application: { ...entry.Resource.application, operator_ids: [operatorId] },
    },
  };
}

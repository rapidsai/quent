// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { TimelineRequest, TaskFilter } from '@quent/utils';

/** Extract the resource_type_name from a TimelineRequest (empty string for Resource requests) */
export function getResourceTypeName(params: TimelineRequest<TaskFilter> | undefined): string {
  if (!params) return '';
  if ('ResourceGroup' in params) return params.ResourceGroup.resource_type_name;
  return '';
}

/** Extract the entity_type_name (FSM filter) from a TimelineRequest */
export function getFsmTypeName(params: TimelineRequest<TaskFilter>): string | null {
  if ('ResourceGroup' in params) return params.ResourceGroup.entity_filter.entity_type_name;
  return params.Resource.entity_filter.entity_type_name;
}

/** Clone entries and set operator_id on each TimelineRequest */
export function setOperatorOnEntry(
  entry: TimelineRequest<TaskFilter>,
  operatorId: string
): TimelineRequest<TaskFilter> {
  if ('ResourceGroup' in entry) {
    return {
      ResourceGroup: {
        ...entry.ResourceGroup,
        app_params: { ...entry.ResourceGroup.app_params, operator_id: operatorId },
      },
    };
  }
  return {
    Resource: {
      ...entry.Resource,
      application: { ...entry.Resource.application, operator_id: operatorId },
    },
  };
}

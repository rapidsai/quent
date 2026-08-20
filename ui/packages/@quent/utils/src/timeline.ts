// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { OperatorFilter, TimelineRequest } from './types';

export function getResourceTypeName(request: TimelineRequest<OperatorFilter> | undefined): string {
  if (!request || !('ResourceGroup' in request)) return '';
  return request.ResourceGroup.resource_type_name;
}

export function getFsmTypeName(request: TimelineRequest<OperatorFilter>): string | null {
  return 'ResourceGroup' in request
    ? request.ResourceGroup.entity_filter.entity_type_name
    : request.Resource.entity_filter.entity_type_name;
}

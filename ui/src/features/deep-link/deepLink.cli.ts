// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function mergeResourceFilter(
  candidate: Record<string, unknown>,
  resourceFilter: Record<string, unknown>
): Record<string, unknown> {
  if (Object.keys(resourceFilter).length === 0) {
    return candidate;
  }

  const resources = isRecord(candidate.resources) ? candidate.resources : {};
  const existingFilter = isRecord(resources.resourceFilter) ? resources.resourceFilter : {};
  return {
    ...candidate,
    resources: {
      ...resources,
      resourceFilter: { ...existingFilter, ...resourceFilter },
    },
  };
}

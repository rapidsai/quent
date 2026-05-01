// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/** Canonical color for each DAG operation type. Used by node rendering and the minimap. */
export const OPERATION_TYPE_COLORS: Record<string, string> = {
  source: '#3b82f6',
  scan: '#3b82f6',
  filesystemscan: '#3b82f6',
  join: '#a855f7',
  joinlocal: '#a855f7',
  joinpartition: '#a855f7',
  aggregate: '#22c55e',
  exchange: '#f97316',
  output: '#ef4444',
  stage: '#6366f1',
  local: '#f59e0b',
  project: '#14b8a6',
  filter: '#06b6d4',
  sort: '#8b5cf6',
  limit: '#ec4899',
  union: '#10b981',
  other: '#6b7280',
};

export const DEFAULT_OPERATION_COLOR = OPERATION_TYPE_COLORS.other;

export function getOperatorColor(operatorType: string): string {
  return OPERATION_TYPE_COLORS[operatorType] ?? DEFAULT_OPERATION_COLOR;
}

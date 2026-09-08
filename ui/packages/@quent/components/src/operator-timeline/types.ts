// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { StatValue } from '../services/query-plan/types';

/**
 * One operator with an active span, normalized for chart consumption.
 * Time is ms elapsed from query start (same domain as resource timelines).
 */
export type OperatorActiveSpanEntry = {
  operatorId: string;
  /** Display name (instance name or type name). */
  label: string;
  /** Operator type name (e.g. "Scan", "Join"). */
  typeName: string;
  startMs: number;
  endMs: number;
  /** Row index for categorical y-axis (0-based). */
  rowIndex: number;
  /** Plan ID this operator belongs to. */
  planId: string;
  /** Pre-computed custom statistics for the operator popup. */
  statistics: Array<{ key: string; value: StatValue; quantity?: string }>;
};

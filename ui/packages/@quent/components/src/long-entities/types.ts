// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DynamicAttribute } from '@quent/utils';

/**
 * One state span within an entity, styled like a timeline mark.
 * Time is milliseconds elapsed from query start.
 */
export type LongEntitySegment = {
  stateName: string;
  startMs: number;
  endMs: number;
  /** State color from the FSM palette. */
  color: string;
  attributes?: DynamicAttribute[];
  derivedAttributes?: DynamicAttribute[];
};

/**
 * One entity (FSM) as a Gantt bar, subdivided into state-colored segments.
 * The bar spans from its first to its last transition.
 */
export type LongEntityEntry = {
  entityId: string;
  /** Display name (instance name or id). */
  label: string;
  /** FSM type name. */
  typeName: string;
  startMs: number;
  endMs: number;
  /** Row index for the categorical y-axis (0-based), assigned by stacking. */
  rowIndex: number;
  segments: LongEntitySegment[];
};

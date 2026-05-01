// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DataFrame, Field } from '@grafana/data';
import type { StatValue } from '@quent/utils';

/**
 * One row per operator. Mirrors the shape consumed by `PivotedStatTable`,
 * but populated from a Grafana `DataFrame` rather than a Quent `QueryBundle`,
 * so any datasource that produces these columns can drive the panel.
 */
export interface OperatorRow {
  partitionId: string;
  partitionLabel: string;
  scopeId: string;
  scopeLabel: string;
  itemType: string;
  itemName: string;
  itemId: string;
  stats: Record<string, StatValue>;
}

/**
 * Reserved field names — anything matching becomes part of the row identity
 * instead of a stat. Field names are matched case-insensitively. Both
 * snake_case and camelCase variants are accepted so authors can use whichever
 * convention their datasource produces.
 */
const IDENTITY_FIELDS = {
  partitionId: ['partition_id', 'partitionid'],
  partitionLabel: ['partition_label', 'partitionlabel'],
  scopeId: ['scope_id', 'scopeid'],
  scopeLabel: ['scope_label', 'scopelabel'],
  itemId: ['item_id', 'itemid'],
  itemType: ['item_type', 'itemtype'],
  itemName: ['item_name', 'itemname'],
} as const;

const IDENTITY_FIELD_SET = new Set<string>(
  Object.values(IDENTITY_FIELDS).flat()
);

function findField(frame: DataFrame, aliases: readonly string[]): Field | undefined {
  for (const f of frame.fields) {
    if (aliases.includes(f.name.toLowerCase())) return f;
  }
  return undefined;
}

function fieldValue(field: Field | undefined, rowIdx: number): unknown {
  if (!field) return undefined;
  // Grafana 11+ exposes Field.values as a plain array; older builds used a
  // Vector with .get(). Support both so this adapter is portable.
  const v = field.values as unknown;
  if (Array.isArray(v)) return v[rowIdx];
  if (v && typeof (v as { get?: (i: number) => unknown }).get === 'function') {
    return (v as { get: (i: number) => unknown }).get(rowIdx);
  }
  return undefined;
}

function asString(value: unknown, fallback: string): string {
  if (value == null) return fallback;
  return String(value);
}

function asStat(value: unknown): StatValue {
  if (value == null) return null;
  if (typeof value === 'number' || typeof value === 'string' || typeof value === 'boolean') {
    return value;
  }
  return String(value);
}

/**
 * Convert all rows in `frames` into `OperatorRow[]`. Concatenates rows from
 * every frame in order, so a query that returns multiple series is fine —
 * each one contributes its rows.
 *
 * Required column: `item_id` (or `itemId`). Without it the row is skipped.
 * All other identity columns fall back sensibly:
 *
 * - `partition_id` defaults to `'-'` (one synthetic partition).
 * - `partition_label` defaults to `partition_id`.
 * - `scope_id` / `scope_label` default to the partition values.
 * - `item_type` defaults to `'-'`.
 * - `item_name` defaults to `item_id`.
 *
 * Every other field becomes a stat keyed by the field's name as-is.
 */
export function frameToOperatorRows(frames: DataFrame[] | undefined): OperatorRow[] {
  if (!frames || frames.length === 0) return [];
  const rows: OperatorRow[] = [];

  for (const frame of frames) {
    if (frame.length === 0) continue;

    const partitionIdField = findField(frame, IDENTITY_FIELDS.partitionId);
    const partitionLabelField = findField(frame, IDENTITY_FIELDS.partitionLabel);
    const scopeIdField = findField(frame, IDENTITY_FIELDS.scopeId);
    const scopeLabelField = findField(frame, IDENTITY_FIELDS.scopeLabel);
    const itemIdField = findField(frame, IDENTITY_FIELDS.itemId);
    const itemTypeField = findField(frame, IDENTITY_FIELDS.itemType);
    const itemNameField = findField(frame, IDENTITY_FIELDS.itemName);

    const statFields = frame.fields.filter(
      f => !IDENTITY_FIELD_SET.has(f.name.toLowerCase())
    );

    for (let i = 0; i < frame.length; i++) {
      const itemIdRaw = fieldValue(itemIdField, i);
      if (itemIdRaw == null || itemIdRaw === '') continue;
      const itemId = String(itemIdRaw);

      const partitionId = asString(fieldValue(partitionIdField, i), '-');
      const partitionLabel = asString(fieldValue(partitionLabelField, i), partitionId);
      const scopeId = asString(fieldValue(scopeIdField, i), partitionId);
      const scopeLabel = asString(fieldValue(scopeLabelField, i), scopeId);
      const itemType = asString(fieldValue(itemTypeField, i), '-');
      const itemName = asString(fieldValue(itemNameField, i), itemId);

      const stats: Record<string, StatValue> = {};
      for (const f of statFields) {
        stats[f.name] = asStat(fieldValue(f, i));
      }

      rows.push({
        partitionId,
        partitionLabel,
        scopeId,
        scopeLabel,
        itemType,
        itemName,
        itemId,
        stats,
      });
    }
  }

  return rows;
}

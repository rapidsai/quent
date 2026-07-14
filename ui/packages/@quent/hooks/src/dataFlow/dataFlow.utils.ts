// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Pure helpers for the DAG data-flow overlay. All response parsing is isolated
// here so the rest of the UI works with normalized, pre-indexed structures.
// Everything is server-declared: state names/order come from the query
// bundle's FSM type declarations, dimension keys and measures from the
// response's `DistributionDecl` — no hardcoded semantics.

import type {
  DataFlowTimelineBinned,
  DataFlowTimelineResponse,
  DistributionDecl,
  FsmTypeDecl,
  QuantitySpec,
  ZoomRange,
} from '@quent/utils';
import { formatQuantity } from '@quent/utils';

/** Bin configuration of the current data-flow window (all values in seconds). */
export interface DataFlowBinConfig {
  startS: number;
  endS: number;
  binDurationS: number;
  numBins: number;
}

/**
 * Presentation metadata for the data-flow overlay, derived once per response.
 */
export interface DataFlowMeta {
  decl: DistributionDecl;
  /**
   * FSM type declaration referenced by `decl.entity_type_name` (from the
   * query bundle), when present. Drives state colors so they match the
   * timeline view.
   */
  fsmType: FsmTypeDecl | null;
  /**
   * Ordered state names: FSM declaration order, filtered to states present in
   * the data. Falls back to sorted data keys when the declaration is missing.
   */
  stateNames: string[];
  bin: DataFlowBinConfig;
  /**
   * Per-measure max operator total across ALL bins of the window — keeps the
   * bar scale stable while scrubbing.
   */
  windowMax: Record<string, number>;
  /** Quantity specs (from the query bundle) keyed by quantity name. */
  quantitySpecs: { [key in string]?: QuantitySpec };
}

/** Per-operator distribution values at one bin. */
export interface DataFlowOperatorFrame {
  /** Sum over all states and dimension keys. */
  total: number;
  /** Totals indexed by `DataFlowMeta.stateNames` order. */
  byState: number[];
  /** Totals indexed by `decl.dimension_keys` order. */
  byDimension: number[];
  /** Values indexed `[stateIndex][dimensionIndex]`. */
  matrix: number[][];
}

/** Snapshot of the data-flow distribution at the playhead's bin. */
export interface DataFlowFrame {
  binIndex: number;
  /** Start time of the bin, in seconds relative to the query epoch. */
  timeS: number;
  /** The measure this frame was extracted for. */
  measure: string;
  /** Window max for the measure (see {@link DataFlowMeta.windowMax}). */
  maxTotal: number;
  /** Operators with a non-zero total at this bin. */
  perOperator: Map<string, DataFlowOperatorFrame>;
}

/**
 * Normalize the externally-tagged response. Returns `null` for
 * `"Unsupported"` or malformed values.
 */
export function normalizeDataFlowResponse(
  response: DataFlowTimelineResponse | null | undefined
): DataFlowTimelineBinned | null {
  if (!response || response === 'Unsupported') return null;
  if (typeof response !== 'object' || !('Binned' in response)) return null;
  return response.Binned;
}

/** Whether the feature should be shown at all: supported and non-empty. */
export function isDataFlowAvailable(
  response: DataFlowTimelineResponse | null | undefined
): boolean {
  const binned = normalizeDataFlowResponse(response);
  return binned != null && Object.keys(binned.operators).length > 0;
}

/**
 * Resolve the request window: the zoom range when valid (end > start),
 * otherwise the full query duration.
 */
export function resolveDataFlowWindow(
  zoom: ZoomRange | null | undefined,
  durationS: number
): { start: number; end: number } {
  if (zoom && zoom.end > zoom.start) return { start: zoom.start, end: zoom.end };
  return { start: 0, end: durationS };
}

/** Map a time (seconds) to a bin index, clamped into `[0, numBins - 1]`. */
export function timeToBinIndex(timeS: number, bin: DataFlowBinConfig): number {
  if (bin.numBins <= 0 || !(bin.binDurationS > 0)) return 0;
  const raw = Math.floor((timeS - bin.startS) / bin.binDurationS);
  return Math.min(bin.numBins - 1, Math.max(0, raw));
}

/** Extract the bin configuration, converting `num_bins` (possibly bigint) to number. */
export function extractBinConfig(binned: DataFlowTimelineBinned): DataFlowBinConfig {
  const { span, bin_duration, num_bins } = binned.config;
  return {
    startS: span.start,
    endS: span.end,
    binDurationS: bin_duration,
    numBins: Number(num_bins),
  };
}

/**
 * Ordered state names for display: FSM declaration order filtered to states
 * present in the data; states missing from the declaration (or the whole
 * declaration missing) fall back to sorted data-key order.
 */
export function resolveDataFlowStates(
  binned: DataFlowTimelineBinned,
  fsmType: FsmTypeDecl | null | undefined
): string[] {
  const present = new Set<string>();
  for (const series of Object.values(binned.operators)) {
    for (const states of Object.values(series.values)) {
      for (const state of Object.keys(states)) present.add(state);
    }
  }
  const declared = fsmType?.states.map(s => s.name) ?? [];
  const ordered = declared.filter(name => present.has(name));
  const orderedSet = new Set(ordered);
  const extras = [...present].filter(s => !orderedSet.has(s)).sort();
  return [...ordered, ...extras];
}

/**
 * Max operator total (summed over states and dimension keys) across all bins
 * of the window for one measure. Missing entries count as zero.
 */
export function computeWindowMax(binned: DataFlowTimelineBinned, measure: string): number {
  const numBins = Number(binned.config.num_bins);
  let max = 0;
  for (const series of Object.values(binned.operators)) {
    const states = series.values[measure];
    if (!states) continue;
    const totals = new Array<number>(numBins).fill(0);
    for (const dims of Object.values(states)) {
      for (const values of Object.values(dims)) {
        const len = Math.min(values.length, numBins);
        for (let i = 0; i < len; i++) totals[i] += values[i]!;
      }
    }
    for (const t of totals) {
      if (t > max) max = t;
    }
  }
  return max;
}

/**
 * Extract the per-operator frame at `binIndex` for `measure`. Operators with
 * an all-zero (or absent) distribution at the bin are omitted from
 * `perOperator`. Missing states/dimension keys read as zero.
 */
export function extractDataFlowFrame(
  binned: DataFlowTimelineBinned,
  stateNames: string[],
  measure: string,
  binIndex: number,
  maxTotal: number
): DataFlowFrame {
  const bin = extractBinConfig(binned);
  const clamped = Math.min(Math.max(binIndex, 0), Math.max(bin.numBins - 1, 0));
  const dimensionKeys = binned.decl.dimension_keys.map(k => k.key);
  const perOperator = new Map<string, DataFlowOperatorFrame>();

  for (const [operatorId, series] of Object.entries(binned.operators)) {
    const states = series.values[measure];
    if (!states) continue;
    const matrix = stateNames.map(() => dimensionKeys.map(() => 0));
    let total = 0;
    stateNames.forEach((state, stateIndex) => {
      const dims = states[state];
      if (!dims) return;
      dimensionKeys.forEach((dimension, dimensionIndex) => {
        const value = dims[dimension]?.[clamped] ?? 0;
        matrix[stateIndex]![dimensionIndex] = value;
        total += value;
      });
    });
    if (total <= 0) continue;
    const byState = matrix.map(row => row.reduce((acc, v) => acc + v, 0));
    const byDimension = dimensionKeys.map((_, dimensionIndex) =>
      matrix.reduce((acc, row) => acc + row[dimensionIndex]!, 0)
    );
    perOperator.set(operatorId, { total, byState, byDimension, matrix });
  }

  return {
    binIndex: clamped,
    timeS: bin.startS + clamped * bin.binDurationS,
    measure,
    maxTotal,
    perOperator,
  };
}

/** Build the presentation metadata for one normalized response. */
export function buildDataFlowMeta(
  binned: DataFlowTimelineBinned,
  fsmTypes: { [key in string]?: FsmTypeDecl } | undefined,
  quantitySpecs: { [key in string]?: QuantitySpec } | undefined
): DataFlowMeta {
  const fsmType = fsmTypes?.[binned.decl.entity_type_name] ?? null;
  const windowMax: Record<string, number> = {};
  for (const measure of binned.decl.measures) {
    windowMax[measure.name] = computeWindowMax(binned, measure.name);
  }
  return {
    decl: binned.decl,
    fsmType,
    stateNames: resolveDataFlowStates(binned, fsmType),
    bin: extractBinConfig(binned),
    windowMax,
    quantitySpecs: quantitySpecs ?? {},
  };
}

/**
 * Resolve the effective measure: the selected one when it is declared,
 * otherwise the first declared measure (or `null` when none exist).
 */
export function resolveDataFlowMeasure(
  selected: string | null,
  decl: DistributionDecl
): string | null {
  if (selected != null && decl.measures.some(m => m.name === selected)) return selected;
  return decl.measures[0]?.name ?? null;
}

/**
 * Format a data-flow value using the measure's declared quantity spec.
 * Values are span-weighted per-bin averages, so fractional counts are
 * expected — defaults to one decimal place.
 */
export function formatDataFlowValue(
  value: number,
  measureName: string,
  meta: DataFlowMeta,
  decimals: number = 1
): string {
  const measure = meta.decl.measures.find(m => m.name === measureName);
  const spec = measure ? meta.quantitySpecs[measure.quantity] : undefined;
  if (measure && spec) return formatQuantity(value, spec, measure.kind, decimals);
  return value.toFixed(decimals);
}

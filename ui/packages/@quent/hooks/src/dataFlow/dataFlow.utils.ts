// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Pure helpers for the DAG data-flow overlay. All response parsing is isolated
// here so the rest of the UI works with normalized, pre-indexed structures.
// Everything is server-declared: state names/order come from the query
// bundle's FSM type declarations, dimension keys and measures from the
// response's `CategoricalDecl` — no hardcoded semantics.

import type {
  CategoricalDecl,
  DataFlowTimelineBinned,
  FsmTypeDecl,
  QuantitySpec,
  ZoomRange,
} from '@quent/utils';
import { formatCompactWithPrefix, formatQuantity, formatQuantityCompact } from '@quent/utils';

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
  decl: CategoricalDecl;
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
   * bar scale stable while scrubbing. Computed over
   * {@link DataFlowMeta.dimensionSelection} only, so bar scaling stays honest
   * when tiers are filtered out.
   */
  windowMax: Record<string, number>;
  /**
   * Effective dimension-key (tier) selection: the user's selection
   * intersected with the declared keys, falling back to ALL declared keys
   * when the selection is `null`, empty, or entirely stale.
   */
  dimensionSelection: ReadonlySet<string>;
  /** Quantity specs (from the query bundle) keyed by quantity name. */
  quantitySpecs: { [key in string]?: QuantitySpec };
}

/**
 * Per-operator distribution values at one bin. All values cover only the
 * SELECTED dimension keys — unselected dimension columns read as zero.
 */
export interface DataFlowOperatorFrame {
  /** Sum over all states and selected dimension keys. */
  total: number;
  /** Totals indexed by `DataFlowMeta.stateNames` order. */
  byState: number[];
  /** Totals indexed by `decl.dimension_keys` order. */
  byDimension: number[];
  /** Values indexed `[stateIndex][dimensionIndex]`. */
  matrix: number[][];
  /**
   * Per-state totals for {@link DataFlowFrame.labelMeasure} (same shape as
   * `byState`; the SAME array instance when the label measure equals the bar
   * measure). Drives in-segment labels while `byState` drives widths.
   */
  labelByState: number[];
  /**
   * Per-dimension totals for {@link DataFlowFrame.labelMeasure} (same shape
   * as `byDimension`; the SAME array instance when the label measure equals
   * the bar measure).
   */
  labelByDimension: number[];
}

/** Snapshot of the data-flow distribution at the playhead's bin. */
export interface DataFlowFrame {
  binIndex: number;
  /** Start time of the bin, in seconds relative to the query epoch. */
  timeS: number;
  /** The measure this frame was extracted for (drives segment widths). */
  measure: string;
  /**
   * The measure driving in-segment value labels. Equals {@link measure}
   * unless the user picked an independent label measure.
   */
  labelMeasure: string;
  /** Window max for the measure (see {@link DataFlowMeta.windowMax}). */
  maxTotal: number;
  /** Operators with a non-zero total at this bin. */
  perOperator: Map<string, DataFlowOperatorFrame>;
  /**
   * Per-operator totals at this bin for EVERY declared measure (not just the
   * selected one), summed over the SELECTED dimension keys only — drives the
   * per-node totals label. Only measures with a non-zero total are present;
   * operators that are zero across all measures are omitted entirely.
   */
  totalsByMeasure: Map<string, Record<string, number>>;
  /**
   * GLOBAL per-tier totals at this bin for EVERY declared measure: the sum
   * over ALL operators and states per dimension key, indexed by
   * `decl.dimension_keys` order. Unlike the rest of the frame this ignores
   * the tier selection — deselected tiers keep their true totals so the
   * legend can annotate dimmed entries too. Every declared measure has an
   * entry (possibly all zeros).
   */
  dimensionTotalsByMeasure: Record<string, number[]>;
}

/**
 * Normalize the response sentinel: the endpoint returns the binned timeline
 * directly, and `null` (unsupported analyzer — HTTP 501) or `undefined` (not
 * yet loaded) both collapse to `null`.
 */
export function normalizeDataFlowResponse(
  response: DataFlowTimelineBinned | null | undefined
): DataFlowTimelineBinned | null {
  return response ?? null;
}

/** Whether the feature should be shown at all: supported and non-empty. */
export function isDataFlowAvailable(response: DataFlowTimelineBinned | null | undefined): boolean {
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
 * Effective dimension-key (tier) selection against the declared keys.
 * `null`, empty, and entirely-stale selections (no overlap with the declared
 * keys, e.g. right after a query/engine switch) all resolve to ALL declared
 * keys — "nothing selected" is never a valid rendering state.
 */
export function resolveDataFlowDimensions(
  selected: ReadonlySet<string> | null | undefined,
  dimensionKeys: string[]
): ReadonlySet<string> {
  if (selected && selected.size > 0) {
    const valid = dimensionKeys.filter(k => selected.has(k));
    if (valid.length > 0) return new Set(valid);
  }
  return new Set(dimensionKeys);
}

/**
 * Max operator total (summed over states and the SELECTED dimension keys)
 * across all bins of the window for one measure. Missing entries count as
 * zero. `selectedDimensions` follows {@link resolveDataFlowDimensions}
 * semantics (`null`/empty/stale = all declared keys).
 */
export function computeWindowMax(
  binned: DataFlowTimelineBinned,
  measure: string,
  selectedDimensions?: ReadonlySet<string> | null
): number {
  const numBins = Number(binned.config.num_bins);
  const selected = resolveDataFlowDimensions(
    selectedDimensions,
    binned.decl.dimension_keys.map(k => k.key)
  );
  let max = 0;
  for (const series of Object.values(binned.operators)) {
    const states = series.values[measure];
    if (!states) continue;
    const totals = new Array<number>(numBins).fill(0);
    for (const dims of Object.values(states)) {
      for (const [dimension, values] of Object.entries(dims)) {
        if (!selected.has(dimension)) continue;
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

/** Optional knobs for {@link extractDataFlowFrame}. */
export interface ExtractDataFlowFrameOptions {
  /**
   * Measure driving in-segment labels (`labelByState`/`labelByDimension`).
   * Defaults to the bar `measure` — the label arrays then alias
   * `byState`/`byDimension`, adding zero cost per scrub tick.
   */
  labelMeasure?: string;
  /**
   * Dimension keys (tiers) to include; follows
   * {@link resolveDataFlowDimensions} semantics (`null`/empty/stale = all).
   * Unselected dimension columns read as zero everywhere in the frame.
   */
  selectedDimensions?: ReadonlySet<string> | null;
}

/**
 * Extract the per-operator frame at `binIndex` for `measure`. Operators with
 * an all-zero (or absent) distribution at the bin are omitted from
 * `perOperator`. Missing states/dimension keys read as zero.
 *
 * Also computes {@link DataFlowFrame.totalsByMeasure} — per-operator totals
 * for every declared measure at the bin — the global
 * {@link DataFlowFrame.dimensionTotalsByMeasure} per-tier totals, and the
 * per-state/per-dimension totals of the label measure (a single cheap pass,
 * recomputed per scrub tick).
 */
export function extractDataFlowFrame(
  binned: DataFlowTimelineBinned,
  stateNames: string[],
  measure: string,
  binIndex: number,
  maxTotal: number,
  options: ExtractDataFlowFrameOptions = {}
): DataFlowFrame {
  const labelMeasure = options.labelMeasure ?? measure;
  const bin = extractBinConfig(binned);
  const clamped = Math.min(Math.max(binIndex, 0), Math.max(bin.numBins - 1, 0));
  const dimensionKeys = binned.decl.dimension_keys.map(k => k.key);
  const selected = resolveDataFlowDimensions(options.selectedDimensions, dimensionKeys);
  const measureNames = binned.decl.measures.map(m => m.name);
  const perOperator = new Map<string, DataFlowOperatorFrame>();
  const totalsByMeasure = new Map<string, Record<string, number>>();
  const dimensionTotalsByMeasure: Record<string, number[]> = {};
  for (const measureName of measureNames) {
    dimensionTotalsByMeasure[measureName] = dimensionKeys.map(() => 0);
  }

  for (const [operatorId, series] of Object.entries(binned.operators)) {
    // Totals at this bin for every declared measure (selected or not):
    // per-operator sums over the selected dimension keys only, plus the
    // global per-tier sums over ALL dimension keys (legend totals must
    // survive tier deselection).
    const totals: Record<string, number> = {};
    let hasAnyMeasure = false;
    for (const measureName of measureNames) {
      const measureStates = series.values[measureName];
      if (!measureStates) continue;
      const dimensionTotals = dimensionTotalsByMeasure[measureName]!;
      let measureTotal = 0;
      for (const state of stateNames) {
        const dims = measureStates[state];
        if (!dims) continue;
        for (let dimensionIndex = 0; dimensionIndex < dimensionKeys.length; dimensionIndex++) {
          const dimension = dimensionKeys[dimensionIndex]!;
          const value = dims[dimension]?.[clamped] ?? 0;
          dimensionTotals[dimensionIndex]! += value;
          if (selected.has(dimension)) measureTotal += value;
        }
      }
      if (measureTotal > 0) {
        totals[measureName] = measureTotal;
        hasAnyMeasure = true;
      }
    }
    if (hasAnyMeasure) totalsByMeasure.set(operatorId, totals);

    const states = series.values[measure];
    if (!states) continue;
    const matrix = stateNames.map(() => dimensionKeys.map(() => 0));
    let total = 0;
    stateNames.forEach((state, stateIndex) => {
      const dims = states[state];
      if (!dims) return;
      dimensionKeys.forEach((dimension, dimensionIndex) => {
        if (!selected.has(dimension)) return;
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

    // Label-measure sums: alias the bar-measure arrays when the measures
    // coincide, otherwise one extra states × dims pass (still trivial).
    let labelByState = byState;
    let labelByDimension = byDimension;
    if (labelMeasure !== measure) {
      labelByState = stateNames.map(() => 0);
      labelByDimension = dimensionKeys.map(() => 0);
      const labelStates = series.values[labelMeasure];
      if (labelStates) {
        stateNames.forEach((state, stateIndex) => {
          const dims = labelStates[state];
          if (!dims) return;
          dimensionKeys.forEach((dimension, dimensionIndex) => {
            if (!selected.has(dimension)) return;
            const value = dims[dimension]?.[clamped] ?? 0;
            labelByState[stateIndex]! += value;
            labelByDimension[dimensionIndex]! += value;
          });
        });
      }
    }

    perOperator.set(operatorId, {
      total,
      byState,
      byDimension,
      matrix,
      labelByState,
      labelByDimension,
    });
  }

  return {
    binIndex: clamped,
    timeS: bin.startS + clamped * bin.binDurationS,
    measure,
    labelMeasure,
    maxTotal,
    perOperator,
    totalsByMeasure,
    dimensionTotalsByMeasure,
  };
}

/**
 * Build the presentation metadata for one normalized response.
 * `selectedDimensions` (the tier selection) shapes `windowMax` and is
 * exposed resolved as `dimensionSelection` — the meta is rebuilt when the
 * selection changes, which is rare (a user click), never per scrub tick.
 */
export function buildDataFlowMeta(
  binned: DataFlowTimelineBinned,
  fsmTypes: { [key in string]?: FsmTypeDecl } | undefined,
  quantitySpecs: { [key in string]?: QuantitySpec } | undefined,
  selectedDimensions?: ReadonlySet<string> | null
): DataFlowMeta {
  const fsmType = fsmTypes?.[binned.decl.entity_type_name] ?? null;
  const dimensionSelection = resolveDataFlowDimensions(
    selectedDimensions,
    binned.decl.dimension_keys.map(k => k.key)
  );
  const windowMax: Record<string, number> = {};
  for (const measure of binned.decl.measures) {
    windowMax[measure.name] = computeWindowMax(binned, measure.name, dimensionSelection);
  }
  return {
    decl: binned.decl,
    fsmType,
    stateNames: resolveDataFlowStates(binned, fsmType),
    bin: extractBinConfig(binned),
    windowMax,
    dimensionSelection,
    quantitySpecs: quantitySpecs ?? {},
  };
}

/**
 * Resolve the effective measure: the selected one when it is declared,
 * otherwise the analyzer-declared `decl.default_measure` (when it names a
 * declared measure), otherwise the first declared measure (or `null` when
 * none exist). An explicit valid selection always wins over the default.
 */
export function resolveDataFlowMeasure(
  selected: string | null,
  decl: CategoricalDecl
): string | null {
  const isDeclared = (name: string) => decl.measures.some(m => m.name === name);
  if (selected != null && isDeclared(selected)) return selected;
  if (decl.default_measure != null && isDeclared(decl.default_measure)) {
    return decl.default_measure;
  }
  return decl.measures[0]?.name ?? null;
}

/**
 * Resolve the effective label measure: the selected one when it is declared,
 * otherwise the bar measure (`null` selection = "follow the bar's measure").
 */
export function resolveDataFlowLabelMeasure(
  selected: string | null,
  decl: CategoricalDecl,
  barMeasure: string
): string {
  if (selected != null && decl.measures.some(m => m.name === selected)) return selected;
  return barMeasure;
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

/**
 * Compact form of {@link formatDataFlowValue} for tight spaces (in-segment
 * labels, per-node totals): 2–3 significant digits, prefix + unit symbol
 * only, no space — e.g. "482", "3.2", "1.2k", "45MiB".
 */
export function formatDataFlowValueCompact(
  value: number,
  measureName: string,
  meta: DataFlowMeta
): string {
  const measure = meta.decl.measures.find(m => m.name === measureName);
  const spec = measure ? meta.quantitySpecs[measure.quantity] : undefined;
  if (measure && spec) return formatQuantityCompact(value, spec, measure.kind);
  return formatCompactWithPrefix(value, '', 'None');
}

/**
 * Estimated pixels per character for in-bar labels (~8px font, tabular
 * digits) — deliberately conservative so labels never overflow their segment.
 */
export const DATA_FLOW_LABEL_CHAR_PX = 6;
/** Horizontal breathing room required around an in-bar label, in pixels. */
export const DATA_FLOW_LABEL_PAD_PX = 4;

/**
 * Width-gated label for one segment of the node flow bars.
 *
 * The bar's filled width is `total / maxTotal` of the track and each segment
 * is flex-sized by `value / total`, so the segment's on-screen width is
 * `(value / maxTotal) * trackPx` — computable purely from frame data, no DOM
 * measurement. Returns the compact label when it fits at
 * ~{@link DATA_FLOW_LABEL_CHAR_PX}px per character (plus
 * {@link DATA_FLOW_LABEL_PAD_PX}px of padding), `null` when the segment is
 * too narrow.
 *
 * When `label` is given, the rendered TEXT comes from `label.value` in
 * `label.measure` (the independent label measure) while the segment WIDTH —
 * and therefore the fit check's available space — stays on `value` in the
 * bar's measure. A zero/absent label value yields `null` (no "0" clutter in
 * segments that only have bar-measure data).
 */
export function fitDataFlowSegmentLabel(
  value: number,
  maxTotal: number,
  measureName: string,
  meta: DataFlowMeta,
  trackPx: number,
  label?: { value: number; measure: string }
): string | null {
  const labelValue = label ? label.value : value;
  const labelMeasure = label ? label.measure : measureName;
  if (!(value > 0) || !(labelValue > 0) || !(maxTotal > 0) || !(trackPx > 0)) return null;
  const segmentPx = (value / maxTotal) * trackPx;
  const text = formatDataFlowValueCompact(labelValue, labelMeasure, meta);
  const requiredPx = text.length * DATA_FLOW_LABEL_CHAR_PX + DATA_FLOW_LABEL_PAD_PX;
  return segmentPx >= requiredPx ? text : null;
}

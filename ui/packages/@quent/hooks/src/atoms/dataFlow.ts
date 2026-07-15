// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// PRIVATE to @quent/hooks — do not export raw atoms (HOOKS-02).
// Consumers use the selector hooks exported from @quent/hooks index.ts.
//
// The raw data-flow response stays in react-query; only the derived
// meta/frame structures live in atoms (written by `useDataFlowSync`).

import { atom } from 'jotai';
import type { DataFlowFrame, DataFlowMeta } from '../dataFlow/dataFlow.utils';

/** User toggle for the data-flow overlay (defaults to on). */
export const dataFlowEnabledAtom = atom(true);

/**
 * Playhead time in seconds relative to the query epoch.
 * `null` = uninitialized — `useDataFlowSync` snaps it to the window start.
 */
export const playheadTimeSAtom = atom<number | null>(null);

/** Selected measure name; `null` falls back to the first declared measure. */
export const selectedDataFlowMeasureAtom = atom<string | null>(null);

/**
 * Measure used for the in-segment value labels of the node flow bars,
 * independent of the measure that sizes the bars. `null` follows the bar's
 * selected measure ({@link selectedDataFlowMeasureAtom}).
 */
export const dataFlowLabelMeasureAtom = atom<string | null>(null);

/**
 * Dimension keys (tiers) included in the data-flow overlay. `null` = all
 * declared keys. Selections that are empty or reference only unknown keys
 * are treated as "all" defensively (the DAGControls chips additionally
 * prevent unchecking the last selected key). `useDataFlowSync` resets this
 * to `null` whenever the declared key set changes (query/engine switch).
 */
export const dataFlowSelectedDimensionsAtom = atom<ReadonlySet<string> | null>(null);

/**
 * Presentation metadata for the current response (decls, bin config,
 * per-measure window max). `null` when the feature is unavailable.
 */
export const dataFlowMetaAtom = atom<DataFlowMeta | null>(null);

/**
 * Frame at the playhead's bin. Only leaf components (NodeFlowBar, info
 * panel) subscribe to this — a scrub tick must not re-render DAG nodes.
 */
export const dataFlowFrameAtom = atom<DataFlowFrame | null>(null);

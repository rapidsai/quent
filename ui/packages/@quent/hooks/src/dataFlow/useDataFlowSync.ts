// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { startTransition, useEffect, useMemo } from 'react';
import { useStore } from 'jotai';
import type { DataFlowTimelineResponse, EntityRef, QueryBundle } from '@quent/utils';
import {
  buildDataFlowMeta,
  extractDataFlowFrame,
  normalizeDataFlowResponse,
  resolveDataFlowMeasure,
  timeToBinIndex,
} from './dataFlow.utils';
import {
  dataFlowFrameAtom,
  dataFlowMetaAtom,
  playheadTimeSAtom,
  selectedDataFlowMeasureAtom,
} from '../atoms/dataFlow';

/**
 * Synchronizes the data-flow response into the private data-flow atoms:
 *
 * - writes {@link dataFlowMetaAtom} whenever the response changes
 * - initializes/clamps the playhead into the response window
 * - recomputes {@link dataFlowFrameAtom} on (response, measure, bin) change
 *
 * The raw response stays in react-query — no raw-response atom. Playhead
 * changes are observed via `store.sub`, so the component calling this hook
 * does NOT re-render on scrub; frame writes happen inside
 * `startTransition` so a fast drag stays responsive.
 *
 * @returns whether the feature is available (supported and non-empty).
 */
export function useDataFlowSync({
  response,
  queryBundle,
}: {
  response: DataFlowTimelineResponse | null | undefined;
  queryBundle: QueryBundle<EntityRef> | null | undefined;
}): { available: boolean } {
  const store = useStore();

  const normalized = useMemo(() => normalizeDataFlowResponse(response), [response]);
  const fsmTypes = queryBundle?.entities.fsm_types;
  const quantitySpecs = queryBundle?.quantity_specs;

  const meta = useMemo(
    () =>
      normalized && Object.keys(normalized.operators).length > 0
        ? buildDataFlowMeta(normalized, fsmTypes, quantitySpecs)
        : null,
    [normalized, fsmTypes, quantitySpecs]
  );

  // Publish meta and keep the playhead inside the current window.
  useEffect(() => {
    store.set(dataFlowMetaAtom, meta);
    if (!meta) {
      store.set(dataFlowFrameAtom, null);
      return;
    }
    const playhead = store.get(playheadTimeSAtom);
    const clamped =
      playhead == null
        ? meta.bin.startS
        : Math.min(Math.max(playhead, meta.bin.startS), meta.bin.endS);
    if (clamped !== playhead) store.set(playheadTimeSAtom, clamped);
  }, [meta, store]);

  // Recompute the frame when the playhead crosses a bin boundary or the
  // selected measure changes. Subscribing imperatively (instead of
  // useAtomValue) keeps the host component from re-rendering on scrub.
  useEffect(() => {
    if (!normalized || !meta) return;

    let lastBinIndex = -1;
    let lastMeasure: string | null = null;

    const recompute = () => {
      const measure = resolveDataFlowMeasure(store.get(selectedDataFlowMeasureAtom), meta.decl);
      if (measure == null) {
        lastBinIndex = -1;
        lastMeasure = null;
        store.set(dataFlowFrameAtom, null);
        return;
      }
      const playhead = store.get(playheadTimeSAtom) ?? meta.bin.startS;
      const binIndex = timeToBinIndex(playhead, meta.bin);
      if (binIndex === lastBinIndex && measure === lastMeasure) return;
      lastBinIndex = binIndex;
      lastMeasure = measure;
      const frame = extractDataFlowFrame(
        normalized,
        meta.stateNames,
        measure,
        binIndex,
        meta.windowMax[measure] ?? 0
      );
      startTransition(() => {
        store.set(dataFlowFrameAtom, frame);
      });
    };

    recompute();
    const unsubPlayhead = store.sub(playheadTimeSAtom, recompute);
    const unsubMeasure = store.sub(selectedDataFlowMeasureAtom, recompute);
    return () => {
      unsubPlayhead();
      unsubMeasure();
    };
  }, [normalized, meta, store]);

  return { available: meta != null };
}

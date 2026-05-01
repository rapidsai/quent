// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useId, useMemo, useRef } from 'react';
import { useAtom } from 'jotai';
import type { OnChangeFn, SortingState } from '@tanstack/react-table';
import {
  indexOrderAtomFamily,
  enabledIndicesAtomFamily,
  selectedStatsAtomFamily,
  statOrderAtomFamily,
  aggModeAtomFamily,
  appliedDefaultKeyAtomFamily,
  sortingAtomFamily,
  type AggMode,
} from '../atoms/pivotTable';

const EMPTY_SORTING: SortingState = [];

interface UseStatGroupTableControlsOptions<TIndexKey extends string, TRow = unknown> {
  baseIndexOrder: TIndexKey[];
  defaultEnabled: Record<TIndexKey, boolean>;
  allStatNames: string[];
  defaultAggMode?: AggMode;
  defaultStatSelector?: (allStats: string[]) => string[] | null;
  filterIndexOrder?: (indexOrder: TIndexKey[]) => TIndexKey[];
  /**
   * Stable string identifier for this table. When provided, controls state
   * persists in Jotai atoms scoped to the surrounding provider so it
   * survives unmount (e.g. tab switches). When omitted, a per-instance
   * fallback key is generated and the state is effectively ephemeral.
   */
  persistKey?: string;
  /**
   * Source rows used to detect whether the active grouping actually
   * collapses multiple rows together. When provided alongside
   * `getRowIndexId`, `isAggregating` is computed by checking whether any
   * two rows share the same combination of active index ids — not just by
   * comparing the number of selected vs. visible index keys. This catches
   * the case where some index keys are deselected but the remaining ones
   * still uniquely identify every source row (so aggregation is a no-op).
   */
  rows?: TRow[];
  getRowIndexId?: (row: TRow, indexKey: TIndexKey) => string;
}

export function useStatGroupTableControls<TIndexKey extends string, TRow = unknown>({
  baseIndexOrder,
  defaultEnabled,
  allStatNames,
  defaultAggMode = 'sum',
  defaultStatSelector,
  filterIndexOrder,
  persistKey,
  rows,
  getRowIndexId,
}: UseStatGroupTableControlsOptions<TIndexKey, TRow>) {
  const fallbackKey = useId();
  const key = persistKey ?? fallbackKey;

  const [indexOrderRaw, setIndexOrderRaw] = useAtom(indexOrderAtomFamily(key));
  const [enabledIndicesRaw, setEnabledIndicesRaw] = useAtom(enabledIndicesAtomFamily(key));
  const [selectedStats, setSelectedStats] = useAtom(selectedStatsAtomFamily(key));
  const [statOrder, setStatOrder] = useAtom(statOrderAtomFamily(key));
  const [aggModeRaw, setAggMode] = useAtom(aggModeAtomFamily(key));
  const [appliedDefaultKey, setAppliedDefaultKey] = useAtom(appliedDefaultKeyAtomFamily(key));
  const [sortingRaw, setSortingRaw] = useAtom(sortingAtomFamily(key));

  const indexOrder = (indexOrderRaw as TIndexKey[] | null) ?? baseIndexOrder;
  const enabledIndices = (enabledIndicesRaw as Record<TIndexKey, boolean> | null) ?? defaultEnabled;
  const aggMode = aggModeRaw ?? defaultAggMode;
  const sorting = sortingRaw ?? EMPTY_SORTING;

  const setSorting = useCallback<OnChangeFn<SortingState>>(
    updater => {
      setSortingRaw(prev => {
        const current = prev ?? EMPTY_SORTING;
        return typeof updater === 'function'
          ? (updater as (old: SortingState) => SortingState)(current)
          : updater;
      });
    },
    [setSortingRaw]
  );

  const defaultStatSelectorRef = useRef(defaultStatSelector);

  useEffect(() => {
    defaultStatSelectorRef.current = defaultStatSelector;
  }, [defaultStatSelector]);

  useEffect(() => {
    if (allStatNames.length === 0) return;
    const allStatsKey = allStatNames.join('\0');
    if (appliedDefaultKey === allStatsKey) return;
    setAppliedDefaultKey(allStatsKey);

    const selectedByDefault = defaultStatSelectorRef.current?.(allStatNames);
    if (!selectedByDefault || selectedByDefault.length === 0) {
      setSelectedStats(null);
      setStatOrder(null);
      return;
    }
    const defaults = new Set(selectedByDefault.filter(stat => allStatNames.includes(stat)));
    if (defaults.size === 0) {
      setSelectedStats(null);
      setStatOrder(null);
      return;
    }
    setSelectedStats(defaults);
    const orderedDefaults = allStatNames.filter(stat => defaults.has(stat));
    const rest = allStatNames.filter(stat => !defaults.has(stat));
    setStatOrder([...orderedDefaults, ...rest]);
  }, [allStatNames, appliedDefaultKey, setAppliedDefaultKey, setSelectedStats, setStatOrder]);

  const orderedStatNames = useMemo(() => {
    if (!statOrder) return allStatNames;
    const allSet = new Set(allStatNames);
    const ordered = statOrder.filter(stat => allSet.has(stat));
    for (const stat of allStatNames) {
      if (!ordered.includes(stat)) ordered.push(stat);
    }
    return ordered;
  }, [allStatNames, statOrder]);

  const visibleStats = useMemo(
    () =>
      selectedStats ? orderedStatNames.filter(stat => selectedStats.has(stat)) : orderedStatNames,
    [orderedStatNames, selectedStats]
  );

  const visibleIndexOrder = useMemo(
    () => (filterIndexOrder ? filterIndexOrder(indexOrder) : indexOrder),
    [indexOrder, filterIndexOrder]
  );

  const activeIndexKeys = useMemo(
    () => visibleIndexOrder.filter(indexKey => enabledIndices[indexKey]),
    [visibleIndexOrder, enabledIndices]
  );

  const isAggregating = useMemo(() => {
    // Fallback for callers that don't supply rows: cheap shape-based check.
    if (!rows || !getRowIndexId) {
      return activeIndexKeys.length < visibleIndexOrder.length;
    }
    if (rows.length <= 1) return false;
    // No active keys means every row collapses into a single bucket.
    if (activeIndexKeys.length === 0) return true;
    const seen = new Set<string>();
    for (const row of rows) {
      const bucketKey = activeIndexKeys.map(k => getRowIndexId(row, k)).join('\0');
      if (seen.has(bucketKey)) return true;
      seen.add(bucketKey);
    }
    return false;
  }, [rows, getRowIndexId, activeIndexKeys, visibleIndexOrder]);

  const handleToggleIndex = useCallback(
    (toggleKey: string) => {
      setEnabledIndicesRaw(prev => {
        const current = (prev as Record<TIndexKey, boolean> | null) ?? defaultEnabled;
        return { ...current, [toggleKey]: !current[toggleKey as TIndexKey] };
      });
    },
    [setEnabledIndicesRaw, defaultEnabled]
  );

  const handleReorderIndex = useCallback(
    (fromKey: string, toKey: string) => {
      setIndexOrderRaw(prev => {
        const current = (prev as TIndexKey[] | null) ?? baseIndexOrder;
        const next = [...current];
        const fromIdx = next.indexOf(fromKey as TIndexKey);
        const toIdx = next.indexOf(toKey as TIndexKey);
        if (fromIdx === -1 || toIdx === -1) return current;
        next.splice(fromIdx, 1);
        next.splice(toIdx, 0, fromKey as TIndexKey);
        return next;
      });
    },
    [setIndexOrderRaw, baseIndexOrder]
  );

  const handleToggleStat = useCallback(
    (stat: string) => {
      setSelectedStats(prev => {
        const current = prev ?? new Set(allStatNames);
        const next = new Set(current);
        if (next.has(stat)) next.delete(stat);
        else next.add(stat);
        return next;
      });
    },
    [allStatNames, setSelectedStats]
  );

  const handleSelectAllStats = useCallback(() => setSelectedStats(null), [setSelectedStats]);
  const handleSelectNoStats = useCallback(() => setSelectedStats(new Set()), [setSelectedStats]);

  return {
    aggMode,
    setAggMode,
    selectedStats,
    orderedStatNames,
    visibleStats,
    visibleIndexOrder,
    activeIndexKeys,
    isAggregating,
    enabledIndices,
    handleToggleIndex,
    handleReorderIndex,
    handleToggleStat,
    handleSelectAllStats,
    handleSelectNoStats,
    sorting,
    setSorting,
  };
}

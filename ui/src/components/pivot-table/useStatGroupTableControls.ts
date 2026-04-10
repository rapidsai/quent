import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { AggMode } from './types';

interface UseStatGroupTableControlsOptions<TIndexKey extends string> {
  baseIndexOrder: TIndexKey[];
  defaultEnabled: Record<TIndexKey, boolean>;
  allStatNames: string[];
  defaultAggMode?: AggMode;
  defaultStatSelector?: (allStats: string[]) => string[] | null;
  filterIndexOrder?: (indexOrder: TIndexKey[]) => TIndexKey[];
}

export function useStatGroupTableControls<TIndexKey extends string>({
  baseIndexOrder,
  defaultEnabled,
  allStatNames,
  defaultAggMode = 'sum',
  defaultStatSelector,
  filterIndexOrder,
}: UseStatGroupTableControlsOptions<TIndexKey>) {
  const [indexOrder, setIndexOrder] = useState<TIndexKey[]>(baseIndexOrder);
  const [enabledIndices, setEnabledIndices] = useState<Record<TIndexKey, boolean>>(defaultEnabled);
  const [selectedStats, setSelectedStats] = useState<Set<string> | null>(null);
  const [statOrder, setStatOrder] = useState<string[] | null>(null);
  const [aggMode, setAggMode] = useState<AggMode>(defaultAggMode);
  const defaultStatSelectorRef = useRef(defaultStatSelector);
  const appliedDefaultKeyRef = useRef<string | null>(null);

  useEffect(() => {
    defaultStatSelectorRef.current = defaultStatSelector;
  }, [defaultStatSelector]);

  useEffect(() => {
    if (allStatNames.length === 0) return;
    const allStatsKey = allStatNames.join('\0');
    if (appliedDefaultKeyRef.current === allStatsKey) return;
    appliedDefaultKeyRef.current = allStatsKey;

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
  }, [allStatNames]);

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

  const isAggregating = activeIndexKeys.length < visibleIndexOrder.length;

  const handleToggleIndex = useCallback((key: string) => {
    setEnabledIndices(prev => ({ ...prev, [key]: !prev[key as TIndexKey] }));
  }, []);

  const handleReorderIndex = useCallback((fromKey: string, toKey: string) => {
    setIndexOrder(prev => {
      const next = [...prev];
      const fromIdx = next.indexOf(fromKey as TIndexKey);
      const toIdx = next.indexOf(toKey as TIndexKey);
      if (fromIdx === -1 || toIdx === -1) return prev;
      next.splice(fromIdx, 1);
      next.splice(toIdx, 0, fromKey as TIndexKey);
      return next;
    });
  }, []);

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
    [allStatNames]
  );

  const handleSelectAllStats = useCallback(() => setSelectedStats(null), []);
  const handleSelectNoStats = useCallback(() => setSelectedStats(new Set()), []);

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
  };
}

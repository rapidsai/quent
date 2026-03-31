import { useMemo, useState, useCallback, useEffect } from 'react';
import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import { QueryToolbar } from '@/components/QueryToolbar';
import {
  selectedPlanIdAtom,
  selectedNodeIdsAtom,
  hoveredOperatorIdAtom,
  hoveredStatAtom,
  hoveredOperatorTypeAtom,
  highlightedNodeIdsAtom,
} from '@/atoms/dag';
import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';
import { operatorTypeColor } from '@/services/colors';
import type { AggMode, FlatRow, PivotedRow } from '../pivot-table/types';
import type { PivotTableGroupKeyEntry } from '../pivot-table/types';
import { StatGroupTable } from '../pivot-table/StatGroupTable';
import { type GroupIndexDef } from '../pivot-table/utils';
import { PivotTableToolbar } from '../pivot-table/PivotTableToolbar';
import { parseCustomStatistics } from '@/lib/queryBundle.utils';
import type { QueryEntities } from '~quent/types/QueryEntities';

function buildOperatorFlatRows(entities: QueryEntities, siblingPlanIds: Set<string>): FlatRow[] {
  const rows: FlatRow[] = [];
  const plans = Object.values(entities.plans)
    .filter((p): p is NonNullable<typeof p> => p != null && siblingPlanIds.has(p.id))
    .sort((a, b) => {
      const wA = a.worker_id ?? '';
      const wB = b.worker_id ?? '';
      if (wA !== wB) return wA.localeCompare(wB);
      return a.id.localeCompare(b.id);
    });

  for (const plan of plans) {
    const worker = plan.worker_id ? entities.workers[plan.worker_id] : undefined;
    const workerPart = worker?.instance_name ?? plan.worker_id ?? '-';
    const planPart = plan.instance_name ?? plan.id;
    const partitionLabel = `${workerPart} / ${planPart}`;
    const partitionId = `${plan.worker_id ?? '-'}:${plan.id}`;

    const ops = Object.values(entities.operators)
      .filter((op): op is NonNullable<typeof op> => op != null && op.plan_id === plan.id)
      .sort((a, b) => {
        const typeA = a.operator_type_name ?? '';
        const typeB = b.operator_type_name ?? '';
        if (typeA !== typeB) return typeA.localeCompare(typeB);
        const nameA = a.instance_name ?? a.id;
        const nameB = b.instance_name ?? b.id;
        return nameA.localeCompare(nameB);
      });

    for (const op of ops) {
      const itemName = op.instance_name ?? op.id;
      const itemType = op.operator_type_name ?? '-';
      const parentOps = (op.parent_operator_ids ?? [])
        .map(id => entities.operators[id])
        .filter((p): p is NonNullable<typeof p> => p != null);
      const parentScopeLabel =
        parentOps.length > 0
          ? [
              ...new Set(
                parentOps.map(p =>
                  p.plan_id ? (entities.plans[p.plan_id]?.instance_name ?? '-') : '-'
                )
              ),
            ].join(', ')
          : '-';
      const parentItemType =
        parentOps.length > 0
          ? [...new Set(parentOps.map(p => p.operator_type_name ?? '-'))].join(', ')
          : '-';
      const parentItemName =
        parentOps.length > 0 ? parentOps.map(p => p.instance_name ?? p.id).join(', ') : '-';
      const base = {
        partitionId,
        partitionLabel,
        scopeId: plan.id,
        scopeLabel: planPart,
        parentScopeLabel,
        parentItemType,
        parentItemName,
        itemType,
        itemName,
        itemId: op.id,
      };

      const duration = op.active_span ? op.active_span.end - op.active_span.start : null;
      rows.push({
        ...base,
        statisticName: 'duration_s',
        value: duration !== null ? Number(duration.toFixed(6)) : null,
      });

      for (const stat of parseCustomStatistics(op)) {
        rows.push({ ...base, statisticName: stat.key, value: stat.value });
      }
    }
  }
  return rows;
}

type IndexKey = 'partition' | 'parent_item_type' | 'parent_item' | 'item_type' | 'item';

const GROUP_INDEX_DEFS: Record<IndexKey, Omit<GroupIndexDef, 'key'>> = {
  partition: {
    getId: row => row.partitionId,
    getLabel: row => row.partitionLabel,
  },
  parent_item_type: {
    getId: row => row.parentItemType,
    getLabel: row => row.parentItemType,
  },
  parent_item: {
    getId: row => row.parentItemName,
    getLabel: row => row.parentItemName,
  },
  item_type: {
    getId: row => row.itemType,
    getLabel: row => row.itemType,
  },
  item: {
    getId: row => row.itemId,
    getLabel: row => row.itemName,
  },
};

const INDEX_ORDER: IndexKey[] = [
  'partition',
  'parent_item_type',
  'parent_item',
  'item_type',
  'item',
];

const DEFAULT_ENABLED: Record<IndexKey, boolean> = {
  partition: true,
  parent_item_type: false,
  parent_item: false,
  item_type: true,
  item: true,
};

interface OperatorTableAdapterProps {
  queryBundle: QueryBundle<EntityRef>;
}

export function OperatorTableAdapter({ queryBundle }: OperatorTableAdapterProps) {
  const [selectedPlanId, setSelectedPlanId] = useAtom(selectedPlanIdAtom);
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const [hoveredOperatorId, setHoveredOperatorId] = useAtom(hoveredOperatorIdAtom);
  const [hoveredStat, setHoveredStat] = useAtom(hoveredStatAtom);
  const setHoveredOperatorType = useSetAtom(hoveredOperatorTypeAtom);
  const setHighlightedNodeIds = useSetAtom(highlightedNodeIdsAtom);
  const { entities } = queryBundle;

  // --- config state (owned here, driven by toolbar) ---
  const [indexOrder, setIndexOrder] = useState<IndexKey[]>(INDEX_ORDER);
  const [enabledIndices, setEnabledIndices] = useState<Record<IndexKey, boolean>>(DEFAULT_ENABLED);
  const [selectedStats, setSelectedStats] = useState<Set<string> | null>(null);
  const [statOrder, setStatOrder] = useState<string[] | null>(null);
  const [aggMode, setAggMode] = useState<AggMode>('sum');

  // --- data ---
  const siblingPlanIds = useMemo(() => {
    const selected = selectedPlanId ? entities.plans[selectedPlanId] : undefined;
    if (!selected) return new Set<string>();
    const parentId = selected.parent;
    const ids = new Set<string>();
    for (const p of Object.values(entities.plans)) {
      if (p && p.parent === parentId) ids.add(p.id);
    }
    return ids;
  }, [entities.plans, selectedPlanId]);

  const flatRows = useMemo(
    () => buildOperatorFlatRows(entities, siblingPlanIds),
    [entities, siblingPlanIds]
  );

  const itemsByParentType = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const row of flatRows) {
      if (row.parentItemType === '-') continue;
      let set = map.get(row.parentItemType);
      if (!set) {
        set = new Set();
        map.set(row.parentItemType, set);
      }
      set.add(row.itemId);
    }
    return map;
  }, [flatRows]);

  const itemsByParentName = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const row of flatRows) {
      if (row.parentItemName === '-') continue;
      let set = map.get(row.parentItemName);
      if (!set) {
        set = new Set();
        map.set(row.parentItemName, set);
      }
      set.add(row.itemId);
    }
    return map;
  }, [flatRows]);

  // --- derived config ---
  const allStatNames = useMemo(() => {
    const seen = new Set<string>();
    const names: string[] = [];
    for (const row of flatRows) {
      if (!seen.has(row.statisticName)) {
        seen.add(row.statisticName);
        names.push(row.statisticName);
      }
    }
    return names;
  }, [flatRows]);

  useEffect(() => {
    if (allStatNames.length === 0) return;
    const duration = allStatNames.filter(s => s === 'duration_s');
    const inputs = allStatNames.filter(s => s.startsWith('input_'));
    const outputs = allStatNames.filter(s => s.startsWith('output_'));
    const defaultNames = [...duration, ...inputs, ...outputs];
    const defaults = new Set(defaultNames);
    if (defaults.size > 0) {
      setSelectedStats(defaults);
      const rest = allStatNames.filter(s => !defaults.has(s));
      setStatOrder([...defaultNames, ...rest]);
    } else {
      setSelectedStats(null);
      setStatOrder(null);
    }
  }, [allStatNames]);

  const orderedStatNames = useMemo(() => {
    if (!statOrder) return allStatNames;
    const allSet = new Set(allStatNames);
    const result = statOrder.filter(s => allSet.has(s));
    for (const s of allStatNames) {
      if (!statOrder.includes(s)) result.push(s);
    }
    return result;
  }, [allStatNames, statOrder]);

  const visibleStats = useMemo(
    () => (selectedStats ? orderedStatNames.filter(s => selectedStats.has(s)) : orderedStatNames),
    [orderedStatNames, selectedStats]
  );

  const hasParentItems = useMemo(() => flatRows.some(r => r.parentItemType !== '-'), [flatRows]);

  const visibleIndexOrder = useMemo(
    () =>
      hasParentItems
        ? indexOrder
        : indexOrder.filter(k => k !== 'parent_item_type' && k !== 'parent_item'),
    [indexOrder, hasParentItems]
  );

  const activeIndexKeys = useMemo(
    () => visibleIndexOrder.filter(k => enabledIndices[k]),
    [visibleIndexOrder, enabledIndices]
  );

  const activeGroupDefs = useMemo(
    () => activeIndexKeys.map(k => ({ key: k, ...GROUP_INDEX_DEFS[k] })),
    [activeIndexKeys]
  );

  const isAggregating = activeIndexKeys.length < visibleIndexOrder.length;

  const parentScopeLabelValue = useMemo(() => {
    for (const row of flatRows) {
      if (row.parentScopeLabel !== '-') return row.parentScopeLabel;
    }
    return 'Parent';
  }, [flatRows]);

  const scopeLabelValue = useMemo(() => {
    for (const row of flatRows) {
      if (row.scopeLabel !== '-') return row.scopeLabel;
    }
    return 'Current';
  }, [flatRows]);

  const indexLabels: Record<string, React.ReactNode> = useMemo(
    () => ({
      partition: 'Worker / Plan',
      parent_item_type: (
        <div>
          <div className="font-mono text-data">{parentScopeLabelValue}</div>
          <div>Operator Type</div>
        </div>
      ),
      parent_item: (
        <div>
          <div className="font-mono text-data">{parentScopeLabelValue}</div>
          <div>Operator Instance</div>
        </div>
      ),
      item_type: (
        <div>
          <div className="font-mono text-data">{scopeLabelValue}</div>
          <div>Operator Type</div>
        </div>
      ),
      item: (
        <div>
          <div className="font-mono text-data">{scopeLabelValue}</div>
          <div>Operator Instance</div>
        </div>
      ),
    }),
    [parentScopeLabelValue, scopeLabelValue]
  );

  const indexConfig = useMemo(
    () =>
      visibleIndexOrder.map(key => ({
        key,
        label: indexLabels[key],
        enabled: enabledIndices[key],
      })),
    [visibleIndexOrder, enabledIndices, indexLabels]
  );

  // --- toolbar callbacks ---
  const handleToggleIndex = useCallback((key: string) => {
    setEnabledIndices(prev => ({ ...prev, [key]: !prev[key as IndexKey] }));
  }, []);

  const handleReorderIndex = useCallback((fromKey: string, toKey: string) => {
    setIndexOrder(prev => {
      const next = [...prev];
      const fromIdx = next.indexOf(fromKey as IndexKey);
      const toIdx = next.indexOf(toKey as IndexKey);
      if (fromIdx === -1 || toIdx === -1) return prev;
      next.splice(fromIdx, 1);
      next.splice(toIdx, 0, fromKey as IndexKey);
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

  const handleReorderStat = useCallback(
    (from: string, to: string) => {
      setStatOrder(prev => {
        const current = prev ?? [...orderedStatNames];
        const next = [...current];
        const fromIdx = next.indexOf(from);
        const toIdx = next.indexOf(to);
        if (fromIdx === -1 || toIdx === -1) return current;
        next.splice(fromIdx, 1);
        next.splice(toIdx, 0, from);
        return next;
      });
    },
    [orderedStatNames]
  );

  const getGroupCellHandlers = useCallback(
    (gk: PivotTableGroupKeyEntry, row: PivotedRow) => {
      const firstItemId = row.itemIds.size === 1 ? [...row.itemIds][0] : null;
      const firstItemScopeId = firstItemId ? row.itemScopeIds.get(firstItemId) : undefined;

      if (gk.key === 'item' && firstItemId) {
        return {
          onMouseEnter: () => {
            if (firstItemScopeId && firstItemScopeId !== selectedPlanId)
              setSelectedPlanId(firstItemScopeId);
            setHoveredOperatorId(firstItemId);
          },
          onMouseLeave: () => {
            if (hoveredOperatorId === firstItemId) setHoveredOperatorId(null);
          },
        };
      }
      if (gk.key === 'parent_item_type') {
        return {
          onMouseEnter: () => setHighlightedNodeIds(itemsByParentType.get(gk.id) ?? null),
          onMouseLeave: () => setHighlightedNodeIds(null),
        };
      }
      if (gk.key === 'parent_item') {
        return {
          onMouseEnter: () => setHighlightedNodeIds(itemsByParentName.get(gk.id) ?? null),
          onMouseLeave: () => setHighlightedNodeIds(null),
        };
      }
      if (gk.key === 'item_type') {
        return {
          onMouseEnter: () => setHoveredOperatorType(gk.id),
          onMouseLeave: () => setHoveredOperatorType(null),
        };
      }
      return {};
    },
    [
      selectedPlanId,
      setSelectedPlanId,
      hoveredOperatorId,
      setHoveredOperatorId,
      setHoveredOperatorType,
      setHighlightedNodeIds,
      itemsByParentType,
      itemsByParentName,
    ]
  );

  if (!selectedPlanId) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        Select a plan on the left to view operators
      </div>
    );
  }

  if (flatRows.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        No operators in the selected plan
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <QueryToolbar />
      <div className="shrink-0 flex flex-col border-b border-border bg-card">
        <PivotTableToolbar
          indexConfig={indexConfig}
          isAggregating={isAggregating}
          aggMode={aggMode}
          allStats={allStatNames}
          orderedStats={orderedStatNames}
          selectedStats={selectedStats}
          onToggleIndex={handleToggleIndex}
          onReorderIndex={handleReorderIndex}
          onSetAggMode={setAggMode}
          onToggleStat={handleToggleStat}
          onSelectAllStats={handleSelectAllStats}
          onSelectNoStats={handleSelectNoStats}
        />
      </div>
      <div className="flex-1 min-h-0">
        <StatGroupTable
          flatRows={flatRows}
          activeIndices={activeGroupDefs}
          visibleStats={visibleStats}
          isAggregating={isAggregating}
          aggMode={aggMode}
          indexLabels={indexLabels}
          selectedItemIds={selectedNodeIds}
          hoveredItemId={hoveredOperatorId}
          hoveredStat={hoveredStat}
          onHoverStat={setHoveredStat}
          getGroupTypeColor={(key, id) =>
            key === 'item_type' || key === 'parent_item_type' ? operatorTypeColor(id) : undefined
          }
          getGroupCellHandlers={getGroupCellHandlers}
          onReorderStat={handleReorderStat}
        />
      </div>
    </div>
  );
}

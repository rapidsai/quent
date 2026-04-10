import { useMemo, useCallback } from 'react';
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
import type { StatValue } from '@/services/query-plan/types';
import type { PivotedRow, StatGroupTableSchema } from '../pivot-table/types';
import type { PivotTableGroupKeyEntry } from '../pivot-table/types';
import { StatGroupTable } from '../pivot-table/StatGroupTable';
import { getSchemaStatNames } from '../pivot-table/utils';
import { PivotTableToolbar } from '../pivot-table/PivotTableToolbar';
import { useStatGroupTableControls } from '../pivot-table/useStatGroupTableControls';
import { parseCustomStatistics } from '@/lib/queryBundle.utils';
import type { QueryEntities } from '~quent/types/QueryEntities';

interface OperatorTableRow {
  partitionId: string;
  partitionLabel: string;
  scopeId: string;
  scopeLabel: string;
  parentScopeLabel: string;
  parentItemType: string;
  parentItemName: string;
  itemType: string;
  itemName: string;
  itemId: string;
  stats: Record<string, StatValue>;
}

function buildOperatorRows(
  entities: QueryEntities,
  siblingPlanIds: Set<string>
): OperatorTableRow[] {
  const rows: OperatorTableRow[] = [];
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
      const duration = op.active_span ? op.active_span.end - op.active_span.start : null;
      const stats: Record<string, StatValue> = {
        duration_s: duration !== null ? Number(duration.toFixed(6)) : null,
      };
      for (const stat of parseCustomStatistics(op)) {
        stats[stat.key] = stat.value;
      }
      rows.push({
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
        stats,
      });
    }
  }
  return rows;
}

type IndexKey = 'partition' | 'parent_item_type' | 'parent_item' | 'item_type' | 'item';

const OPERATOR_SCHEMA: StatGroupTableSchema<OperatorTableRow> = {
  groups: {
    partition: {
      id: row => row.partitionId,
      label: row => row.partitionLabel,
    },
    parent_item_type: {
      id: row => row.parentItemType,
    },
    parent_item: {
      id: row => row.parentItemName,
    },
    item_type: {
      id: row => row.itemType,
    },
    item: {
      id: row => row.itemId,
      label: row => row.itemName,
    },
  },
  itemId: row => row.itemId,
  scopeId: row => row.scopeId,
  itemType: row => row.itemType,
  stats: row => row.stats,
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

  const rows = useMemo(
    () => buildOperatorRows(entities, siblingPlanIds),
    [entities, siblingPlanIds]
  );

  const itemsByParentType = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const row of rows) {
      if (row.parentItemType === '-') continue;
      let set = map.get(row.parentItemType);
      if (!set) {
        set = new Set();
        map.set(row.parentItemType, set);
      }
      set.add(row.itemId);
    }
    return map;
  }, [rows]);

  const itemsByParentName = useMemo(() => {
    const map = new Map<string, Set<string>>();
    for (const row of rows) {
      if (row.parentItemName === '-') continue;
      let set = map.get(row.parentItemName);
      if (!set) {
        set = new Set();
        map.set(row.parentItemName, set);
      }
      set.add(row.itemId);
    }
    return map;
  }, [rows]);

  const allStatNames = useMemo(() => getSchemaStatNames(rows, OPERATOR_SCHEMA), [rows]);
  const hasParentItems = useMemo(() => rows.some(r => r.parentItemType !== '-'), [rows]);
  const filterIndexOrder = useCallback(
    (order: IndexKey[]) =>
      hasParentItems ? order : order.filter(k => k !== 'parent_item_type' && k !== 'parent_item'),
    [hasParentItems]
  );
  const {
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
  } = useStatGroupTableControls<IndexKey>({
    baseIndexOrder: INDEX_ORDER,
    defaultEnabled: DEFAULT_ENABLED,
    allStatNames,
    defaultStatSelector: stats => {
      const duration = stats.filter(stat => stat === 'duration_s');
      const inputs = stats.filter(stat => stat.startsWith('input_'));
      const outputs = stats.filter(stat => stat.startsWith('output_'));
      return [...duration, ...inputs, ...outputs];
    },
    filterIndexOrder,
  });

  const parentScopeLabelValue = useMemo(() => {
    for (const row of rows) {
      if (row.parentScopeLabel !== '-') return row.parentScopeLabel;
    }
    return 'Parent';
  }, [rows]);

  const scopeLabelValue = useMemo(() => {
    for (const row of rows) {
      if (row.scopeLabel !== '-') return row.scopeLabel;
    }
    return 'Current';
  }, [rows]);

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

  if (rows.length === 0) {
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
          rows={rows}
          schema={OPERATOR_SCHEMA}
          activeIndices={activeIndexKeys}
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
        />
      </div>
    </div>
  );
}

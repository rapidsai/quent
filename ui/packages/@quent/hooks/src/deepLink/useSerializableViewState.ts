// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useCallback } from 'react';
import { useStore } from 'jotai';
import type { SortingState } from '@tanstack/react-table';
import type { ContinuousPaletteName, DagLayoutDirection, NodeLabelField } from '@quent/utils';
import { selectedNodeIdsAtom, selectedPlanIdAtom } from '../atoms/dag';
import {
  dataFlowEnabledAtom,
  dataFlowIsPlayingAtom,
  dataFlowLabelMeasureAtom,
  dataFlowSelectedDimensionsAtom,
  playheadLineTimeMsAtom,
  playheadTimeSAtom,
  selectedDataFlowMeasureAtom,
} from '../atoms/dataFlow';
import {
  edgeColorPaletteAtom,
  nodeColorPaletteAtom,
  selectedColorField,
  selectedDagLayoutDirectionAtom,
  selectedEdgeColorFieldAtom,
  selectedEdgeWidthFieldAtom,
  selectedNodeLabelFieldAtom,
} from '../atoms/dagControls';
import {
  aggModeAtomFamily,
  enabledIndicesAtomFamily,
  indexOrderAtomFamily,
  selectedStatsAtomFamily,
  sortingAtomFamily,
  statOrderAtomFamily,
  type AggMode,
} from '../atoms/pivotTable';

export interface SerializableDagControls {
  nodeColorField: string | null;
  nodeColorPalette: ContinuousPaletteName;
  edgeWidthField: string | null;
  edgeColorField: string | null;
  edgeColorPalette: ContinuousPaletteName;
  nodeLabelField: NodeLabelField;
  layoutDirection: DagLayoutDirection;
}

export interface SerializableDataFlowState {
  enabled: boolean;
  measure: string | null;
  labelMeasure: string | null;
  dimensions: string[] | null;
  playheadS: number | null;
}

export interface SerializableOperatorTableState {
  groupingOrder?: string[];
  enabledGroups?: string[];
  visibleStats?: string[];
  aggregation?: AggMode;
  sort?: SortingState;
}

export interface SerializableViewState {
  selection: {
    planId: string;
    operatorNodeIds: string[];
  };
  dag: SerializableDagControls;
  dataFlow: SerializableDataFlowState;
  operatorTable: SerializableOperatorTableState;
}

export interface HydratableViewState {
  selection?: {
    planId?: string;
    operatorNodeIds?: readonly string[];
  };
  dag?: Partial<SerializableDagControls>;
  dataFlow?: Partial<SerializableDataFlowState>;
  operatorTable?: SerializableOperatorTableState;
}

interface SerializableViewStateOptions {
  operatorTablePersistKey: string;
  operatorTableGroupKeys: readonly string[];
}

export function useSerializableViewState({
  operatorTablePersistKey,
  operatorTableGroupKeys,
}: SerializableViewStateOptions) {
  const store = useStore();

  const read = useCallback((): SerializableViewState => {
    const selectedStats = store.get(selectedStatsAtomFamily(operatorTablePersistKey));
    const statOrder = store.get(statOrderAtomFamily(operatorTablePersistKey));
    const orderedStats = statOrder ?? (selectedStats ? [...selectedStats] : null);
    const enabledIndices = store.get(enabledIndicesAtomFamily(operatorTablePersistKey));

    return {
      selection: {
        planId: store.get(selectedPlanIdAtom),
        operatorNodeIds: [...store.get(selectedNodeIdsAtom)].sort(),
      },
      dag: {
        nodeColorField: store.get(selectedColorField),
        nodeColorPalette: store.get(nodeColorPaletteAtom),
        edgeWidthField: store.get(selectedEdgeWidthFieldAtom),
        edgeColorField: store.get(selectedEdgeColorFieldAtom),
        edgeColorPalette: store.get(edgeColorPaletteAtom),
        nodeLabelField: store.get(selectedNodeLabelFieldAtom),
        layoutDirection: store.get(selectedDagLayoutDirectionAtom),
      },
      dataFlow: {
        enabled: store.get(dataFlowEnabledAtom),
        measure: store.get(selectedDataFlowMeasureAtom),
        labelMeasure: store.get(dataFlowLabelMeasureAtom),
        dimensions: store.get(dataFlowSelectedDimensionsAtom)
          ? [...store.get(dataFlowSelectedDimensionsAtom)!].sort()
          : null,
        playheadS: store.get(playheadTimeSAtom),
      },
      operatorTable: {
        ...(store.get(indexOrderAtomFamily(operatorTablePersistKey)) && {
          groupingOrder: store.get(indexOrderAtomFamily(operatorTablePersistKey))!,
        }),
        ...(enabledIndices && {
          enabledGroups: operatorTableGroupKeys.filter(key => enabledIndices[key]),
        }),
        ...(selectedStats &&
          orderedStats && {
            visibleStats: orderedStats.filter(stat => selectedStats.has(stat)),
          }),
        ...(store.get(aggModeAtomFamily(operatorTablePersistKey)) && {
          aggregation: store.get(aggModeAtomFamily(operatorTablePersistKey))!,
        }),
        ...(store.get(sortingAtomFamily(operatorTablePersistKey)) && {
          sort: store.get(sortingAtomFamily(operatorTablePersistKey))!,
        }),
      },
    };
  }, [operatorTableGroupKeys, operatorTablePersistKey, store]);

  const hydrate = useCallback(
    (state: HydratableViewState) => {
      if (state.selection?.planId !== undefined) {
        store.set(selectedPlanIdAtom, state.selection.planId);
      }
      if (state.selection?.operatorNodeIds !== undefined) {
        store.set(selectedNodeIdsAtom, new Set(state.selection.operatorNodeIds));
      }

      const dag = state.dag;
      if (dag?.nodeColorField !== undefined) {
        store.set(selectedColorField, dag.nodeColorField);
      }
      if (dag?.nodeColorPalette !== undefined) {
        store.set(nodeColorPaletteAtom, dag.nodeColorPalette);
      }
      if (dag?.edgeWidthField !== undefined) {
        store.set(selectedEdgeWidthFieldAtom, dag.edgeWidthField);
      }
      if (dag?.edgeColorField !== undefined) {
        store.set(selectedEdgeColorFieldAtom, dag.edgeColorField);
      }
      if (dag?.edgeColorPalette !== undefined) {
        store.set(edgeColorPaletteAtom, dag.edgeColorPalette);
      }
      if (dag?.nodeLabelField !== undefined) {
        store.set(selectedNodeLabelFieldAtom, dag.nodeLabelField);
      }
      if (dag?.layoutDirection !== undefined) {
        store.set(selectedDagLayoutDirectionAtom, dag.layoutDirection);
      }

      const dataFlow = state.dataFlow;
      if (dataFlow?.enabled !== undefined) {
        store.set(dataFlowEnabledAtom, dataFlow.enabled);
      }
      if (dataFlow?.measure !== undefined) {
        store.set(selectedDataFlowMeasureAtom, dataFlow.measure);
      }
      if (dataFlow?.labelMeasure !== undefined) {
        store.set(dataFlowLabelMeasureAtom, dataFlow.labelMeasure);
      }
      if (dataFlow?.dimensions !== undefined) {
        store.set(
          dataFlowSelectedDimensionsAtom,
          dataFlow.dimensions === null ? null : new Set(dataFlow.dimensions)
        );
      }
      if (dataFlow?.playheadS !== undefined) {
        store.set(playheadTimeSAtom, dataFlow.playheadS);
      }
      store.set(dataFlowIsPlayingAtom, false);
      store.set(playheadLineTimeMsAtom, null);

      const table = state.operatorTable;
      if (table?.groupingOrder !== undefined) {
        store.set(indexOrderAtomFamily(operatorTablePersistKey), table.groupingOrder);
      }
      if (table?.enabledGroups !== undefined) {
        const enabled = new Set(table.enabledGroups);
        store.set(
          enabledIndicesAtomFamily(operatorTablePersistKey),
          Object.fromEntries(operatorTableGroupKeys.map(key => [key, enabled.has(key)]))
        );
      }
      if (table?.visibleStats !== undefined) {
        store.set(selectedStatsAtomFamily(operatorTablePersistKey), new Set(table.visibleStats));
        store.set(statOrderAtomFamily(operatorTablePersistKey), table.visibleStats);
      }
      if (table?.aggregation !== undefined) {
        store.set(aggModeAtomFamily(operatorTablePersistKey), table.aggregation);
      }
      if (table?.sort !== undefined) {
        store.set(sortingAtomFamily(operatorTablePersistKey), table.sort);
      }
    },
    [operatorTableGroupKeys, operatorTablePersistKey, store]
  );

  return { read, hydrate } as const;
}

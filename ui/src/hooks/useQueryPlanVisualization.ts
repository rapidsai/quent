// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';
import type { QueryPlanDataItem, DAGNode, DAGEdge } from '@/services/query-plan/types';
import { getTreeData, getPlanDAG } from '@/services/query-plan/query-bundle-transformer';

interface DAGData {
  nodes: DAGNode[];
  edges: DAGEdge[];
  queryData: QueryPlanDataItem[];
}

interface UseQueryPlanVisualizationResult {
  dagData: DAGData;
  treeData: QueryPlanDataItem[];
  error: Error | null;
}

export const useQueryPlanVisualization = (
  queryBundle: QueryBundle<EntityRef> | undefined,
  planId: string
): UseQueryPlanVisualizationResult => {
  const treeData = useMemo(() => {
    try {
      return queryBundle ? getTreeData(queryBundle) : [];
    } catch (error) {
      console.error('Error generating tree data:', error);
      return [];
    }
  }, [queryBundle]);

  const result = useMemo<UseQueryPlanVisualizationResult>(() => {
    if (!queryBundle || !planId) {
      return {
        dagData: { nodes: [], edges: [], queryData: [] },
        treeData,
        error: null,
      };
    }

    try {
      const dag = getPlanDAG(queryBundle, planId);
      return {
        dagData: { ...dag, queryData: treeData },
        treeData,
        error: null,
      };
    } catch (error) {
      const errorObject = error instanceof Error ? error : new Error(String(error));
      return {
        dagData: { nodes: [], edges: [], queryData: [] },
        treeData,
        error: errorObject,
      };
    }
  }, [queryBundle, planId, treeData]);

  return result;
};

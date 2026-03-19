import { useEffect, useMemo, lazy, Suspense } from 'react';
import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import { useQueryBundle } from '@/hooks/useQueryBundle';
import { useQueryPlanVisualization } from '@/hooks/useQueryPlanVisualization';
import { TreeView } from '@/components/ui/tree-view';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable';
import { type QueryPlanDataItem, type NodeColoring, type EdgeWidthConfig } from '@/services/query-plan/types';
import { Network } from 'lucide-react';
import {
  selectedPlanIdAtom,
  hoveredWorkerIdAtom,
  selectedColorField,
  nodeColoringAtom,
  selectedEdgeWidthFieldAtom,
  edgeWidthConfigAtom,
} from '@/atoms/dag';
import { DAGControls } from '@/components/dag/DAGControls';
import { parseCustomStatistics } from '@/lib/queryBundle.utils.ts';
import { getActivePalette } from '@/services/colors';

// Lazy load DAGChart to split elkjs (~1.6MB) into a separate chunk
const DAGChart = lazy(() =>
  import('@/components/dag/DAGChart').then(mod => ({ default: mod.DAGChart }))
);

export function QueryPlan({ queryId, engineId }: { queryId: string; engineId: string }) {
  const [planId, setPlanId] = useAtom(selectedPlanIdAtom);
  const setHoveredWorkerId = useSetAtom(hoveredWorkerIdAtom);

  const {
    data: queryBundle,
    isLoading: queryBundleLoading,
    error: queryBundleError,
  } = useQueryBundle({ engineId, queryId });

  const { dagData, treeData, error: dagError } = useQueryPlanVisualization(queryBundle, planId);
  const statistics = dagData.nodes.map(n => parseCustomStatistics(n.metadata?.rawNode));

  // Node coloring
  const selectedField = useAtomValue(selectedColorField);
  const setNodeColoring = useSetAtom(nodeColoringAtom);

  const nodeColoring = useMemo((): NodeColoring => {
    if (!selectedField || !dagData.nodes.length) return null;
    const entries = dagData.nodes.flatMap(node => {
      const stat = parseCustomStatistics(node.metadata?.rawNode).find(s => s.key === selectedField);
      if (stat?.value == null) return [];
      return [{ id: node.id, value: stat.value }];
    });
    if (!entries.length) return null;

    if (entries.every(e => typeof e.value === 'number')) {
      const nums = entries.map(e => e.value as number);
      return {
        type: 'continuous',
        values: new Map(entries.map(e => [e.id, e.value as number])),
        min: Math.min(...nums),
        max: Math.max(...nums),
      };
    }
    // Categorical: assign palette colors by unique value
    const palette = getActivePalette();
    const uniqueValues = [...new Set(entries.map(e => String(e.value)))];
    const valueColor = new Map(uniqueValues.map((v, i) => [v, palette[i % palette.length]]));
    return {
      type: 'categorical',
      colorMap: new Map(entries.map(e => [e.id, valueColor.get(String(e.value))!])),
    };
  }, [selectedField, dagData.nodes]);

  useEffect(() => { setNodeColoring(nodeColoring); }, [nodeColoring, setNodeColoring]);

  // Edge width
  const selectedEdgeWidthField = useAtomValue(selectedEdgeWidthFieldAtom);
  const setEdgeWidthConfig = useSetAtom(edgeWidthConfigAtom);

  const edgeWidthConfig = useMemo((): EdgeWidthConfig => {
    if (!selectedEdgeWidthField || !dagData.edges.length) return null;
    const entries = dagData.edges.flatMap(edge => {
      const stat = (edge.portStats ?? []).find(s => s.key === selectedEdgeWidthField);
      if (typeof stat?.value !== 'number') return [];
      return [{ id: edge.id, value: stat.value }];
    });
    if (!entries.length) return null;
    const nums = entries.map(e => e.value);
    return {
      values: new Map(entries.map(e => [e.id, e.value])),
      min: Math.min(...nums),
      max: Math.max(...nums),
    };
  }, [selectedEdgeWidthField, dagData.edges]);

  useEffect(() => { setEdgeWidthConfig(edgeWidthConfig); }, [edgeWidthConfig, setEdgeWidthConfig]);

  // Port stat fields for edge width dropdown
  const portStatFields = useMemo(
    () => [...new Set(dagData.edges.flatMap(e => (e.portStats ?? []).map(s => s.key)))],
    [dagData.edges]
  );

  const handlePlanSelect = (item: QueryPlanDataItem | undefined) => {
    if (item) {
      setPlanId(item.id);
    }
  };

  // TODO: Currently fetching root plan when bundle loads - is this correct?
  useEffect(() => {
    if (queryBundle && !planId) {
      setPlanId(queryBundle.plan_tree.id);
    }
  }, [queryBundle, planId, setPlanId]);

  // handle loading and error states
  if (queryBundleLoading) {
    return (
      <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
        <div className="flex justify-center items-center h-full text-muted-foreground">
          Loading query plan...
        </div>
      </div>
    );
  }

  const errorMessage = queryBundleError
    ? `Failed to load query plan: ${queryBundleError instanceof Error ? queryBundleError.message : 'Unknown error'}`
    : dagError
      ? `Failed to generate query plan visualization: ${dagError.message}`
      : null;

  if (errorMessage) {
    return (
      <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
        <div className="flex justify-center items-center h-full text-destructive">
          {errorMessage}
        </div>
      </div>
    );
  }

  if (!queryBundle || !planId) {
    return null;
  }

  const singleQueryPlan = treeData.length === 1 && !treeData[0]?.children;

  const renderItem = ({ item, hasChildren }: { item: QueryPlanDataItem; hasChildren: boolean }) => {
    return (
      <div
        className="flex flex-col items-start py-0.5 pl-1"
        onMouseEnter={() => item.workerId && setHoveredWorkerId(item.workerId)}
        onMouseLeave={() => setHoveredWorkerId(null)}
      >
        {singleQueryPlan ? (
          <span className="text-xs">Query: {item.queryId}</span>
        ) : (
          <span className="text-xs">
            <span className="capitalize">{item.planType}</span>
            {!hasChildren && <span>: {item.id}</span>}
          </span>
        )}
        {item.workerId && (
          <span className="text-xs text-muted-foreground">Worker: {item.workerId}</span>
        )}
        {hasChildren && (
          <span className="text-xs text-muted-foreground capitalize text-left">ID: {item.id}</span>
        )}
      </div>
    );
  };

  return (
    <div className="w-full flex flex-col h-[calc(100vh-4rem)]">
      <div>
        <DAGControls statistics={statistics} portStatFields={portStatFields} />
      </div>
      <div className="flex items-center gap-2 px-4 py-1.5 border-b border-border bg-card flex-shrink-0">
        <Network className="h-4 w-4 text-primary" />
        <h3 className="text-xs font-semibold text-foreground">Query Plan Explorer</h3>
        <div className="text-xs text-muted-foreground">
          {queryBundle.entities.query_group.instance_name} -{' '}
          {queryBundle.entities.query.instance_name}
        </div>
      </div>

      <ResizablePanelGroup orientation="vertical" className="flex-1">
        <ResizablePanel
          defaultSize="20%"
          minSize="10%"
          collapsible
          collapsedSize="0%"
          className="overflow-y-auto [&::-webkit-scrollbar]:w-0 [scrollbar-width:none] [-ms-overflow-style:none]"
        >
          <TreeView<QueryPlanDataItem>
            data={treeData}
            initialSelectedItemId={planId}
            onSelectChange={handlePlanSelect}
            renderItem={renderItem}
          />
        </ResizablePanel>

        <ResizableHandle withHandle data-panel-group-direction="vertical" />

        {/* DAG Chart - lazy loaded to split elkjs into separate chunk */}
        <ResizablePanel
          defaultSize="75%"
          minSize="25%"
          collapsible
          collapsedSize="0%"
          className="overflow-hidden"
        >
          <Suspense
            fallback={
              <div className="flex items-center justify-center h-full text-muted-foreground">
                Loading visualization...
              </div>
            }
          >
            <DAGChart data={dagData} height="100%" />
          </Suspense>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}

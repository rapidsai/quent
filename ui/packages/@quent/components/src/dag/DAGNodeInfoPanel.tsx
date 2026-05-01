// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Panel } from '@xyflow/react';
import { Pin } from 'lucide-react';
import { useSelectedNodeIds, useHoveredNodeData, useSelectedNodeData } from '@quent/hooks';
import { DataText } from '../ui/data-text';

/**
 * Floating panel anchored to the DAG canvas that surfaces details for the
 * hovered node, or, while a node is selected, pins the selection's details.
 */
export const DAGNodeInfoPanel = () => {
  const selectedNodeIds = useSelectedNodeIds();
  const hoveredNodeData = useHoveredNodeData();
  const selectedNodeData = useSelectedNodeData();

  const isPinned = selectedNodeIds.size > 0;
  const displayData = isPinned ? selectedNodeData : hoveredNodeData;

  if (!displayData) return null;
  return (
    <Panel
      position="bottom-left"
      className="nodrag nopan mb-2 ml-2"
      style={isPinned ? undefined : { pointerEvents: 'none' }}
    >
      <div className="w-72 rounded-md border bg-popover p-4 text-popover-foreground shadow-md">
        <div className="flex items-center justify-between gap-2">
          <DataText className="font-semibold text-sm truncate">{displayData.label}</DataText>
          <div className="flex items-center gap-1 flex-shrink-0">
            {isPinned && <Pin className="h-3 w-3 text-muted-foreground" />}
            <DataText className="text-xs text-muted-foreground capitalize px-1.5 py-0.5 bg-muted rounded">
              {displayData.operationType}
            </DataText>
          </div>
        </div>
        <DataText as="div" className="text-xs text-muted-foreground truncate mt-0.5">
          {displayData.nodeId}
        </DataText>
        {displayData.statistics.length > 0 && (
          <div className="mt-1 border-t pt-1.5 max-h-56 overflow-y-auto [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent">
            <div className="flex flex-col gap-1 pr-3">
              {displayData.statistics.map(({ key, value }) => (
                <div key={key} className="text-xs mt-1">
                  {Array.isArray(value) ? (
                    <div className="flex items-center justify-between gap-0.5">
                      <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
                      <div className="ml-2 flex flex-col gap-0.5">
                        {value.map((item, i) => (
                          <DataText key={i} className="text-muted-foreground whitespace-pre-line">
                            {item}
                          </DataText>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <div className="flex items-center justify-between">
                      <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
                      <DataText className="text-muted-foreground ml-1">{String(value)}</DataText>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </Panel>
  );
};

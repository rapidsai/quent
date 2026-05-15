// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { ChevronUp, ChevronDown } from 'lucide-react';
import { useSelectedNodeData } from '@quent/hooks';
import { DataText } from '../ui/data-text';
import { thinScrollbarClass } from '../ui/thin-scroll';
import { inferFieldFormatter } from '../services/query-plan/dagFieldProcessing';

export const DAGNodeInfoPanel = () => {
  const selectedNodeData = useSelectedNodeData();
  const [isExpanded, setIsExpanded] = useState(false);

  useEffect(() => {
    if (!selectedNodeData) {
      setIsExpanded(false);
    }
  }, [selectedNodeData]);

  return (
    <div className="border-t bg-card flex-shrink-0">
      <div className="flex items-center justify-between px-4 py-1.5 min-w-0">
        <div className="flex items-center gap-2 min-w-0 overflow-hidden">
          <span className="text-xs text-muted-foreground font-medium flex-shrink-0">
            Operator Details
          </span>
          {selectedNodeData && (
            <>
              <span className="text-muted-foreground text-xs flex-shrink-0">·</span>
              <DataText className="text-xs font-medium truncate">{selectedNodeData.label}</DataText>
              <DataText className="text-xs text-muted-foreground capitalize px-1.5 py-0.5 bg-muted rounded flex-shrink-0">
                {selectedNodeData.operationType}
              </DataText>
            </>
          )}
        </div>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          disabled={!selectedNodeData}
          className="ml-2 rounded p-1 hover:bg-muted transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-auto disabled:hover:bg-transparent flex-shrink-0"
          aria-label="Toggle operator details"
        >
          {isExpanded ? (
            <ChevronDown className="h-3 w-3 text-muted-foreground" />
          ) : (
            <ChevronUp className="h-3 w-3 text-muted-foreground" />
          )}
        </button>
      </div>

      {isExpanded && selectedNodeData && (
        <div className={`border-t px-4 pb-2 h-48 overflow-y-auto ${thinScrollbarClass}`}>
          <div className="flex flex-col gap-1 pr-2 pt-1.5">
            <div className="text-xs flex items-center justify-between">
              <DataText className="capitalize">ID:</DataText>
              <DataText className="text-muted-foreground ml-1 truncate">
                {selectedNodeData.nodeId}
              </DataText>
            </div>
            {selectedNodeData.statistics?.map(({ key, value }) => (
              <div key={key} className="text-xs">
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
                    <DataText className="text-muted-foreground ml-1">
                      {typeof value === 'number' ? inferFieldFormatter(key)(value) : String(value)}
                    </DataText>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

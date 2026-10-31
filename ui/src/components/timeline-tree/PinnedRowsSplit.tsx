// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useRef, type ReactNode } from 'react';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  useCollapsiblePanel,
} from '@quent/components';
import { cn } from '@quent/utils';
import { measurePinnedPanelHeights } from './pinnedPanelHeights';

// Used for the first paint, before the real height is measured.
const FALLBACK_PANEL_HEIGHT = 240;
const FALLBACK_MIN_HEIGHT = 64;

interface PinnedRowsSplitProps {
  // The pinned rows table. Leave out when nothing is pinned.
  pinned?: ReactNode;
  pinnedHeader?: ReactNode;
  pinnedCount: number;
  isCollapsed: boolean;
  children: ReactNode;
}

/**
 * Shows the pinned rows in a panel above the main table, with a handle between them to
 * resize it. Each panel scrolls on its own.
 */
export function PinnedRowsSplit({
  pinned,
  pinnedHeader,
  pinnedCount,
  isCollapsed,
  children,
}: PinnedRowsSplitProps) {
  const contentRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLDivElement>(null);
  const hasPinned = pinned != null;

  // The panel grows with each pinned row, up to the limit, until the user sizes it themselves.
  const { panelRef, handleProps } = useCollapsiblePanel({
    enabled: hasPinned,
    collapsed: isCollapsed,
    fitKey: pinnedCount,
    measure: () => measurePinnedPanelHeights(contentRef.current, headerRef.current),
  });

  return (
    <ResizablePanelGroup orientation="vertical" className="h-full">
      {hasPinned && (
        <ResizablePanel
          key="pinned"
          id="pinned-rows"
          panelRef={panelRef}
          defaultSize={FALLBACK_PANEL_HEIGHT}
          minSize={FALLBACK_MIN_HEIGHT}
          groupResizeBehavior="preserve-pixel-size"
          className="min-h-0"
        >
          <div ref={contentRef} className="flex h-full flex-col">
            {pinnedHeader != null && (
              <div ref={headerRef} className="shrink-0">
                {pinnedHeader}
              </div>
            )}
            <div className="min-h-0 flex-1">{pinned}</div>
          </div>
        </ResizablePanel>
      )}
      {hasPinned && (
        <ResizableHandle key="pinned-handle" withHandle disabled={isCollapsed} {...handleProps} />
      )}
      <ResizablePanel key="main" id="timeline-tree" minSize="10%" className="min-h-0">
        <div className={cn('h-full', hasPinned && 'pt-4')}>{children}</div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

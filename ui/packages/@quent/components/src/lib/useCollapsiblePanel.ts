// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffectEvent, useLayoutEffect, useRef, type RefObject } from 'react';
import type { PanelImperativeHandle } from 'react-resizable-panels';

// A new panel can't be resized until its group has measured it, and resizing sooner throws.
// Try again each frame until that has happened. Returns a function that stops trying.
const MAX_LAYOUT_ATTEMPTS = 30;
function whenPanelReady(apply: () => void): () => void {
  let frame = 0;
  let attempts = 0;
  const attempt = () => {
    try {
      apply();
    } catch (error) {
      attempts += 1;
      if (attempts >= MAX_LAYOUT_ATTEMPTS) {
        throw error;
      }
      frame = requestAnimationFrame(attempt);
    }
  };
  frame = requestAnimationFrame(attempt);
  return () => cancelAnimationFrame(frame);
}

export interface PanelHeights {
  /** Height that shows everything rendered right now. While collapsed, that is the header. */
  content: number;
  /** Height the panel should open at. */
  fit: number;
}

export interface UseCollapsiblePanelOptions {
  /** False while the panel is not rendered. */
  enabled: boolean;
  collapsed: boolean;
  /** The panel is fitted again whenever this changes, until the user resizes it. */
  fitKey: unknown;
  /** Reads the heights from what is rendered. Return nothing if there is no layout yet. */
  measure: () => PanelHeights | undefined;
  /** Height the user left the panel at, kept by the caller so it survives a remount. */
  savedSize?: number;
  /** Called with the height each time the user resizes the open panel. */
  onSizeChange?: (size: number | undefined) => void;
}

export interface CollapsiblePanel {
  /** Pass to the `panelRef` of the `ResizablePanel`. */
  panelRef: RefObject<PanelImperativeHandle | null>;
  /** Pass to the `onResize` of the `ResizablePanel`, so the user's height can be saved. */
  onResize: (size: { inPixels: number }) => void;
  /** Spread onto the `ResizableHandle`, so a resize by the user stops the automatic fitting. */
  handleProps: { onPointerDown: () => void; onKeyDown: () => void };
}

/**
 * Sizes a panel from its content. It opens at a fitted height, shrinks to its header when
 * collapsed and goes back to its previous height when expanded. Once the user drags the
 * handle, the fitted height is no longer applied.
 */
export function useCollapsiblePanel({
  enabled,
  collapsed,
  fitKey,
  measure,
  savedSize,
  onSizeChange,
}: UseCollapsiblePanelOptions): CollapsiblePanel {
  const panelRef = useRef<PanelImperativeHandle | null>(null);
  const openSizeRef = useRef<number | undefined>(undefined);
  const userSizedRef = useRef(false);
  const measureHeights = useEffectEvent(measure);
  const isCollapsed = useEffectEvent(() => collapsed);
  const getSavedSize = useEffectEvent(() => savedSize);
  const reportSize = useEffectEvent((size: number | undefined) => onSizeChange?.(size));

  useLayoutEffect(() => {
    if (!enabled) {
      openSizeRef.current = undefined;
      userSizedRef.current = false;
      reportSize(undefined);
      return;
    }
    // Back from another tab: the user's height wins over the fitted one.
    const saved = getSavedSize();
    if (saved !== undefined && !userSizedRef.current) {
      userSizedRef.current = true;
      openSizeRef.current = saved;
    }
    if (userSizedRef.current) {
      if (!isCollapsed() && openSizeRef.current) {
        const size = openSizeRef.current;
        return whenPanelReady(() => panelRef.current?.resize(size));
      }
      if (isCollapsed()) {
        const heights = measureHeights();
        const content = heights?.content;
        return content ? whenPanelReady(() => panelRef.current?.resize(content)) : undefined;
      }
      return;
    }
    const heights = measureHeights();
    if (!heights || heights.content === 0) {
      return;
    }
    openSizeRef.current = heights.fit;
    const target = isCollapsed() ? heights.content : heights.fit;
    return whenPanelReady(() => panelRef.current?.resize(target));
  }, [enabled, fitKey]);

  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!panel) {
      return;
    }
    if (collapsed) {
      return whenPanelReady(() => {
        openSizeRef.current = panel.getSize().inPixels;
        const content = measureHeights()?.content;
        if (content) {
          panel.resize(content);
        }
      });
    }
    const openSize = openSizeRef.current;
    return openSize ? whenPanelReady(() => panel.resize(openSize)) : undefined;
  }, [collapsed]);

  const markUserSized = () => {
    userSizedRef.current = true;
  };

  const onResize = ({ inPixels }: { inPixels: number }) => {
    if (userSizedRef.current && !isCollapsed()) {
      openSizeRef.current = inPixels;
      reportSize(inPixels);
    }
  };

  return {
    panelRef,
    onResize,
    handleProps: { onPointerDown: markUserSized, onKeyDown: markUserSized },
  };
}

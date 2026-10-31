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
}

export interface CollapsiblePanel {
  /** Pass to the `panelRef` of the `ResizablePanel`. */
  panelRef: RefObject<PanelImperativeHandle | null>;
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
}: UseCollapsiblePanelOptions): CollapsiblePanel {
  const panelRef = useRef<PanelImperativeHandle | null>(null);
  const openSizeRef = useRef<number | undefined>(undefined);
  const userSizedRef = useRef(false);
  const measureHeights = useEffectEvent(measure);
  const isCollapsed = useEffectEvent(() => collapsed);

  useLayoutEffect(() => {
    if (!enabled) {
      openSizeRef.current = undefined;
      userSizedRef.current = false;
      return;
    }
    if (userSizedRef.current) {
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

  return { panelRef, handleProps: { onPointerDown: markUserSized, onKeyDown: markUserSized } };
}

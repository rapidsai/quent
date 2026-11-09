// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, type RefObject } from 'react';
import { TREE_TABLE_SCROLLER_SELECTOR as SCROLLER_SELECTOR } from '@quent/components';

/**
 * Keeps the horizontal scroll position of every table scroller inside `containerRef` the same,
 * so the header and both body tables line up. Vertical scrolling is left alone.
 * `enabled` lets the caller wait until the scrollers exist.
 */
export function useSyncedHorizontalScroll(
  containerRef: RefObject<HTMLElement | null>,
  enabled: boolean
): void {
  useEffect(() => {
    const container = containerRef.current;
    if (!enabled || !container) {
      return;
    }
    // Scroll events don't bubble, so listen during the capture phase.
    const onScroll = (event: Event) => {
      const source = event.target;
      if (!(source instanceof HTMLElement) || !source.matches(SCROLLER_SELECTOR)) {
        return;
      }
      for (const scroller of container.querySelectorAll<HTMLElement>(SCROLLER_SELECTOR)) {
        // Only write when the value differs, so the mirrored scroll doesn't echo back.
        if (scroller !== source && scroller.scrollLeft !== source.scrollLeft) {
          scroller.scrollLeft = source.scrollLeft;
        }
      }
    };
    container.addEventListener('scroll', onScroll, true);
    return () => container.removeEventListener('scroll', onScroll, true);
  }, [containerRef, enabled]);
}

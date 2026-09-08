// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from 'react';
import { cn } from '@quent/utils';
import { DataText } from './data-text';
import { HoverCard, HoverCardContent, HoverCardTrigger } from './hover-card';

const HOVER_CARD_OPEN_DELAY_MS = 300;

/**
 * Shared overflow-detection + delayed-open behavior for hover-card tooltips
 * triggered by truncated text. Consumers attach `handlePointerEnter`/
 * `handlePointerLeave` to the element referenced by `elementRef` and pass
 * `open` to a `HoverCard`.
 */
export function useOverflowHoverCard<T extends HTMLElement>(
  elementRef: { current: T | null },
  enabled = true
) {
  const [open, setOpen] = useState(false);
  const openTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearOpenTimer = () => {
    if (openTimerRef.current) {
      clearTimeout(openTimerRef.current);
      openTimerRef.current = null;
    }
  };

  const handlePointerEnter = () => {
    clearOpenTimer();
    const element = elementRef.current;
    if (!enabled || !element || element.scrollWidth <= element.clientWidth) {
      return;
    }
    openTimerRef.current = setTimeout(() => setOpen(true), HOVER_CARD_OPEN_DELAY_MS);
  };

  const handlePointerLeave = () => {
    clearOpenTimer();
    setOpen(false);
  };

  useEffect(
    () => () => {
      if (openTimerRef.current) {
        clearTimeout(openTimerRef.current);
      }
    },
    []
  );

  return { open, handlePointerEnter, handlePointerLeave };
}

export function OverflowHoverCardContent({
  label,
  side = 'right',
}: {
  label: string;
  side?: 'top' | 'right' | 'bottom' | 'left';
}) {
  return (
    <HoverCardContent
      side={side}
      align="start"
      className="pointer-events-none w-auto max-w-sm bg-background p-2 text-foreground"
    >
      <DataText className="break-all text-xs">{label}</DataText>
    </HoverCardContent>
  );
}

export function OverflowingItemLabel({ label, className }: { label: string; className?: string }) {
  const triggerRef = useRef<HTMLSpanElement>(null);
  const { open, handlePointerEnter, handlePointerLeave } = useOverflowHoverCard(triggerRef);

  return (
    <HoverCard open={open}>
      <HoverCardTrigger asChild>
        <span
          ref={triggerRef}
          className={cn('min-w-0 flex-1 truncate', className)}
          onPointerEnter={handlePointerEnter}
          onPointerLeave={handlePointerLeave}
          onBlur={handlePointerLeave}
        >
          <DataText>{label}</DataText>
        </span>
      </HoverCardTrigger>
      <OverflowHoverCardContent label={label} />
    </HoverCard>
  );
}

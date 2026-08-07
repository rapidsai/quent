// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

const POINTER_OFFSET = 12;
const VIEWPORT_MARGIN = 4;

export interface PointerPosition {
  clientX: number;
  clientY: number;
}

export function PointerTooltipPortal({
  hover,
  children,
}: {
  hover: PointerPosition | null;
  children: ReactNode;
}) {
  if (!hover) return null;
  return <PositionedPointerTooltip hover={hover}>{children}</PositionedPointerTooltip>;
}

function PositionedPointerTooltip({
  hover,
  children,
}: {
  hover: PointerPosition;
  children: ReactNode;
}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [position, setPosition] = useState({
    left: hover.clientX + POINTER_OFFSET,
    top: hover.clientY + POINTER_OFFSET,
  });

  useLayoutEffect(() => {
    const element = hostRef.current;
    if (!element) return;
    const rect = element.getBoundingClientRect();
    let left = hover.clientX + POINTER_OFFSET;
    let top = hover.clientY + POINTER_OFFSET;
    if (left + rect.width + VIEWPORT_MARGIN > window.innerWidth) {
      left = Math.max(VIEWPORT_MARGIN, hover.clientX - rect.width - POINTER_OFFSET);
    }
    if (top + rect.height + VIEWPORT_MARGIN > window.innerHeight) {
      top = Math.max(VIEWPORT_MARGIN, hover.clientY - rect.height - POINTER_OFFSET);
    }
    setPosition({ left, top });
  }, [hover.clientX, hover.clientY, children]);

  return createPortal(
    <div
      ref={hostRef}
      className="pointer-events-none fixed z-[1000]"
      style={{ left: position.left, top: position.top }}
    >
      {children}
    </div>,
    document.body
  );
}

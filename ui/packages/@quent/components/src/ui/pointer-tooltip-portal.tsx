// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { PositionedTooltip } from './positioned-tooltip';

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
  return (
    <PositionedTooltip clientX={hover.clientX} clientY={hover.clientY}>
      {children}
    </PositionedTooltip>
  );
}

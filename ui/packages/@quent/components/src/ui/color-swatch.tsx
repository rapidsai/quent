// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn } from '@quent/utils';

type ColorSwatchProps = {
  color: string;
  shape?: 'circle' | 'square';
  className?: string;
};

export function ColorSwatch({ color, shape = 'circle', className }: ColorSwatchProps) {
  return (
    <span
      aria-hidden
      className={cn(
        'inline-block h-2 w-2 shrink-0',
        shape === 'circle' ? 'rounded-full' : 'rounded-sm',
        className
      )}
      style={{ backgroundColor: color }}
    />
  );
}

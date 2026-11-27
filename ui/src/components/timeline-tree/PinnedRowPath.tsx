// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Fragment } from 'react';
import { ChevronRight } from 'lucide-react';
import { DataText } from '@quent/components';

const PATH_SEPARATOR = ' > ';

interface PinnedRowPathProps {
  // Names of the rows above the pinned row, outermost first.
  names: readonly string[];
}

// Where a pinned row sits in the tree, like "worker > resource group". When the column is too
// narrow, each name is cut off with its own ellipsis so every part of the path stays visible,
// and the full path shows on hover.
export function PinnedRowPath({ names }: PinnedRowPathProps) {
  return (
    <span
      className="flex max-w-full items-center"
      title={names.join(PATH_SEPARATOR)}
      data-testid="pinned-row-path"
    >
      {names.map((name, index) => (
        <Fragment key={index}>
          {index > 0 && (
            <ChevronRight aria-hidden className="mx-0.5 h-3 w-3 shrink-0 -translate-y-px" />
          )}
          <DataText className="min-w-0 truncate text-foreground">{name}</DataText>
        </Fragment>
      ))}
    </span>
  );
}

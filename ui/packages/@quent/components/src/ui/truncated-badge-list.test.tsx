// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Badge } from './badge';
import { TruncatedBadgeList } from './truncated-badge-list';

describe('TruncatedBadgeList', () => {
  it('lists hidden item labels in the overflow badge title', () => {
    render(
      <TruncatedBadgeList
        items={['Alpha', 'Beta', 'Gamma']}
        maxVisible={1}
        getItemKey={item => item}
        getItemLabel={item => item}
        renderBadge={item => <Badge>{item}</Badge>}
      />
    );

    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.queryByText('Beta')).not.toBeInTheDocument();
    expect(screen.getByText('+2 more')).toHaveAttribute('title', 'Beta, Gamma');
  });
});

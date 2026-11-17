// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import { QueryPlanNode } from './QueryPlanNode';

describe('QueryPlanNode', () => {
  it('renders its border above the preview action', () => {
    const { container } = render(
      <Provider store={createStore()}>
        <QueryPlanNode
          data={{
            label: 'Scan',
            nodeId: 'scan',
            operationType: 'scan',
            actions: { filtered: false, details: {} as never },
          }}
        />
      </Provider>
    );

    expect(screen.getByRole('button', { name: 'Preview details for Scan' })).toBeVisible();
    const border = container.querySelector(
      'span.pointer-events-none.absolute.-inset-px[style*="border-color"]'
    );
    expect(border).toHaveClass('z-30', 'border');
    expect(border?.getAttribute('style')).toContain('border-color: inherit');
  });
});

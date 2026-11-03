// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { InnerScrollTable } from './inner-scroll-table';
import { TableBody, TableCell, TableHead, TableHeader, TableRow } from './table';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('InnerScrollTable', () => {
  it('positions multiple sticky columns without pinning spanning rows', () => {
    vi.spyOn(HTMLTableCellElement.prototype, 'getBoundingClientRect').mockImplementation(function (
      this: HTMLTableCellElement
    ) {
      const width = Number(this.dataset.testWidth ?? 0);
      return {
        width,
        height: 40,
        x: 0,
        y: 0,
        top: 0,
        right: width,
        bottom: 40,
        left: 0,
        toJSON: () => ({}),
      };
    });

    render(
      <InnerScrollTable stickyLeftColumns={2} stickyRightColumns={2}>
        <TableHeader>
          <TableRow>
            {[100, 120, 140, 160, 180].map((width, index) => (
              <TableHead key={width} data-test-width={width}>
                Header {index}
              </TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow>
            <TableCell colSpan={5}>Group</TableCell>
          </TableRow>
          <TableRow>
            {Array.from({ length: 5 }, (_, index) => (
              <TableCell key={index}>Cell {index}</TableCell>
            ))}
          </TableRow>
        </TableBody>
      </InnerScrollTable>
    );

    const headers = screen.getAllByRole('columnheader');
    expect(headers[0]).toHaveAttribute('data-inner-scroll-sticky-left');
    expect(headers[0]).toHaveStyle({ left: '0px' });
    expect(headers[1]).toHaveAttribute('data-inner-scroll-sticky-left');
    expect(headers[1]).toHaveStyle({ left: '100px' });
    expect(headers[2]).not.toHaveAttribute('data-inner-scroll-sticky');
    expect(headers[3]).toHaveAttribute('data-inner-scroll-sticky-right');
    expect(headers[3]).toHaveStyle({ right: '180px' });
    expect(headers[4]).toHaveAttribute('data-inner-scroll-sticky-right');
    expect(headers[4]).toHaveStyle({ right: '0px' });
    expect(screen.getByText('Group')).not.toHaveAttribute('data-inner-scroll-sticky');
  });
});

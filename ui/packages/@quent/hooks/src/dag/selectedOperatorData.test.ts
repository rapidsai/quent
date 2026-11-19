// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vite-plus/test';
import type { SelectedOperatorGroupData } from '@quent/utils';
import { removeSelectedOperatorData, upsertSelectedOperatorData } from './selectedOperatorData';

const scan: SelectedOperatorGroupData = {
  nodeId: 'scan',
  label: 'Scan',
  operationType: 'scan',
  statistics: [],
};

const join: SelectedOperatorGroupData = {
  nodeId: 'join',
  label: 'Join',
  operationType: 'join',
  statistics: [],
};

describe('selected operator data', () => {
  it('adds a second operator without replacing the first', () => {
    const first = upsertSelectedOperatorData(new Map(), 'scan', scan);
    const second = upsertSelectedOperatorData(first, 'join', join);

    expect([...second.keys()]).toEqual(['scan', 'join']);
    expect(second.get('scan')).toEqual(scan);
    expect(second.get('join')).toEqual(join);
  });

  it('keys selected data by selection ID instead of operator ID', () => {
    const result = upsertSelectedOperatorData(new Map(), 'logical-join', join);

    expect(result).toEqual(new Map([['logical-join', join]]));
  });

  it('removes only the requested operator', () => {
    const both = upsertSelectedOperatorData(
      upsertSelectedOperatorData(new Map(), 'scan', scan),
      'join',
      join
    );

    expect(removeSelectedOperatorData(both, 'scan')).toEqual(new Map([['join', join]]));
  });
});

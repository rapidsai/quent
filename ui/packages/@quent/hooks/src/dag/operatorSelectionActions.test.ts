// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createStore } from 'jotai';
import { describe, expect, it } from 'vitest';
import { operatorSelectionActionAtom, operatorSelectionAtom } from '../atoms/dag';
import { selectedOperatorsDataAtom } from '../atoms/dagControls';
import { pinnedOperatorDetailsAtom } from '../atoms/operatorDetails';

const scanData = {
  nodeId: 'scan',
  label: 'Scan',
  operationType: 'scan',
  statistics: [],
};

const joinData = {
  nodeId: 'join',
  label: 'Join',
  operationType: 'join',
  statistics: [],
};

const groupedJoinData = {
  ...joinData,
  relatedOperators: [scanData],
};

describe('operator selection actions', () => {
  it('updates filter and selected operator data together', () => {
    const store = createStore();

    const selectedIds = store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'logical-scan',
      label: 'Scan',
      operatorIds: ['logical-scan', 'physical-scan'],
      selectedData: scanData,
    });

    expect(selectedIds).toEqual(new Set(['logical-scan', 'physical-scan']));
    expect(store.get(operatorSelectionAtom).selections.has('logical-scan')).toBe(true);
    expect(store.get(selectedOperatorsDataAtom)).toEqual(new Map([['logical-scan', scanData]]));
    expect(store.get(pinnedOperatorDetailsAtom)).toEqual(new Map([['logical-scan', scanData]]));

    store.set(operatorSelectionActionAtom, { type: 'remove', selectionId: 'logical-scan' });

    expect(store.get(operatorSelectionAtom).selections.size).toBe(0);
    expect(store.get(selectedOperatorsDataAtom).size).toBe(0);
    expect(store.get(pinnedOperatorDetailsAtom).size).toBe(0);
  });

  it('keeps only the parent data when parent and child selections overlap', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'scan',
      label: 'Scan',
      operatorIds: ['scan'],
      selectedData: scanData,
    });

    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'join',
      label: 'Join',
      operatorIds: ['join', 'scan'],
      selectedData: groupedJoinData,
    });

    expect([...store.get(operatorSelectionAtom).selections.keys()]).toEqual(['join']);
    expect(store.get(selectedOperatorsDataAtom)).toEqual(new Map([['join', groupedJoinData]]));

    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'scan',
      label: 'Scan',
      operatorIds: ['scan'],
      selectedData: scanData,
    });

    expect([...store.get(operatorSelectionAtom).selections.keys()]).toEqual(['join']);
    expect(store.get(selectedOperatorsDataAtom)).toEqual(new Map([['join', groupedJoinData]]));
  });

  it('promotes related operator data when a parent selection is split', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'join',
      label: 'Join',
      operatorIds: ['join', 'scan'],
      selectedData: groupedJoinData,
    });

    store.set(operatorSelectionActionAtom, {
      type: 'replace',
      selections: [{ selectionId: 'scan', label: 'Scan', operatorIds: new Set(['scan']) }],
    });

    expect(store.get(operatorSelectionAtom).selections.has('join')).toBe(false);
    expect(store.get(selectedOperatorsDataAtom)).toEqual(new Map([['scan', scanData]]));
  });

  it('prunes stale operator data when replacing or clearing selections', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'scan',
      label: 'Scan',
      operatorIds: ['scan'],
      selectedData: scanData,
    });
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'join',
      label: 'Join',
      operatorIds: ['join'],
      selectedData: joinData,
    });

    store.set(operatorSelectionActionAtom, {
      type: 'replace',
      selections: [{ selectionId: 'scan', label: 'Scan', operatorIds: new Set(['scan']) }],
    });

    expect([...store.get(selectedOperatorsDataAtom).keys()]).toEqual(['scan']);

    store.set(operatorSelectionActionAtom, { type: 'clear' });

    expect(store.get(operatorSelectionAtom).selections.size).toBe(0);
    expect(store.get(selectedOperatorsDataAtom).size).toBe(0);
  });

  it('keys a grouped replacement by selection ID when its selected operator differs', () => {
    const store = createStore();

    store.set(operatorSelectionActionAtom, {
      type: 'replace',
      selections: [
        {
          selectionId: 'logical-join',
          label: 'Join',
          operatorIds: new Set(['logical-join', 'physical-join']),
          selectedData: joinData,
        },
      ],
    });

    expect(store.get(operatorSelectionAtom).selections).toEqual(
      new Map([
        [
          'logical-join',
          {
            label: 'Join',
            operatorIds: new Set(['logical-join', 'physical-join']),
          },
        ],
      ])
    );
    expect(store.get(selectedOperatorsDataAtom)).toEqual(new Map([['logical-join', joinData]]));
  });

  it('hydrates selected operator data without changing global selections', () => {
    const store = createStore();
    store.set(operatorSelectionActionAtom, {
      type: 'replace',
      selections: [
        {
          selectionId: 'join',
          label: 'Join',
          operatorIds: new Set(['join', 'physical-join']),
        },
        {
          selectionId: 'unknown',
          label: 'Unknown operator',
          operatorIds: new Set(['unknown']),
        },
      ],
    });

    store.set(operatorSelectionActionAtom, {
      type: 'hydrate',
      selections: [
        {
          selectionId: 'join',
          label: 'Join',
          operatorIds: ['join', 'physical-join'],
          selectedData: groupedJoinData,
        },
        {
          selectionId: 'physical-join',
          label: 'Physical join',
          operatorIds: ['physical-join'],
          selectedData: scanData,
        },
      ],
    });

    expect(store.get(operatorSelectionAtom).selections).toEqual(
      new Map([
        [
          'join',
          {
            label: 'Join',
            operatorIds: new Set(['join', 'physical-join']),
          },
        ],
        [
          'unknown',
          {
            label: 'Unknown operator',
            operatorIds: new Set(['unknown']),
          },
        ],
      ])
    );
    expect(store.get(selectedOperatorsDataAtom)).toEqual(new Map([['join', groupedJoinData]]));
  });

  it('keeps selected operators that are not present in the loaded DAG', () => {
    const store = createStore();
    const timelineData = {
      nodeId: 'timeline-only',
      label: 'Timeline operator',
      operationType: 'scan',
      statistics: [],
    };
    store.set(operatorSelectionActionAtom, {
      type: 'add',
      selectionId: 'timeline-only',
      label: 'Timeline operator',
      operatorIds: ['timeline-only'],
      selectedData: timelineData,
    });

    store.set(operatorSelectionActionAtom, {
      type: 'hydrate',
      selections: [],
    });

    expect(store.get(operatorSelectionAtom).selections.get('timeline-only')).toEqual({
      label: 'Timeline operator',
      operatorIds: new Set(['timeline-only']),
    });
    expect(store.get(selectedOperatorsDataAtom)).toEqual(
      new Map([['timeline-only', timelineData]])
    );
  });
});

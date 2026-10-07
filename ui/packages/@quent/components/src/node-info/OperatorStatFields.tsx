// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import {
  useDisplayedPipes,
  useSetHoveredPipeId,
  useSetInspectedPipe,
  type SelectedOperatorData,
} from '@quent/hooks';
import { isStatStruct, type QuantitySpec } from '@quent/utils';
import { StatisticFields, type StatisticSectionAction } from './StatisticFields';

export const OperatorStatFields = ({
  operator,
  quantitySpecs,
}: {
  operator: SelectedOperatorData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) => {
  const edges = useDisplayedPipes();
  const setHoveredPipe = useSetHoveredPipeId();
  const setInspectedPipe = useSetInspectedPipe();
  useEffect(() => () => setHoveredPipe(null), [operator.nodeId, setHoveredPipe]);
  const actions = new Map<string, StatisticSectionAction>();
  const normalize = (name: string) => name.toLowerCase().replace(/[^a-z0-9]/g, '');
  for (const section of operator.statistics) {
    if ((section.key !== 'Inputs' && section.key !== 'Outputs') || !isStatStruct(section.value)) {
      continue;
    }
    const input = section.key === 'Inputs';
    for (const port of section.value.fields) {
      if (!isStatStruct(port.value)) {
        continue;
      }
      const role = port.value.fields.find(field => field.key === 'structural_role')?.value;
      const name = typeof role === 'string' ? role : port.key;
      const matches = edges.filter(
        edge =>
          (input ? edge.target : edge.source) === operator.nodeId &&
          normalize((input ? edge.targetPortName : edge.sourcePortName) ?? '') === normalize(name)
      );
      const edge = matches.length === 1 ? matches[0] : undefined;
      if (!edge?.sourcePortId || !edge.targetPortId) {
        continue;
      }
      const ref = { sourcePortId: edge.sourcePortId, targetPortId: edge.targetPortId };
      actions.set(JSON.stringify([section.key, port.key]), {
        label: `Inspect ${section.key.toLowerCase()} ${port.key} pipe`,
        enter: () => setHoveredPipe(edge.id),
        leave: () => setHoveredPipe(null),
        activate: () => {
          setHoveredPipe(null);
          setInspectedPipe(ref);
        },
      });
    }
  }
  return (
    <>
      <dl className="mb-2 grid grid-cols-[auto_minmax(0,1fr)] gap-2 text-xs">
        <dt>ID</dt>
        <dd className="truncate text-muted-foreground" title={operator.nodeId}>
          {operator.nodeId}
        </dd>
      </dl>
      <StatisticFields
        statistics={[...(operator.attributes ?? []), ...operator.statistics]}
        quantitySpecs={quantitySpecs}
        sectionActions={actions}
      />
    </>
  );
};

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { SelectedOperatorData } from '@quent/hooks';
import type { QuantitySpec } from '@quent/utils';
import { StatisticFields } from './StatisticFields';

export const OperatorStatFields = ({
  operator,
  quantitySpecs,
}: {
  operator: SelectedOperatorData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) => (
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
    />
  </>
);

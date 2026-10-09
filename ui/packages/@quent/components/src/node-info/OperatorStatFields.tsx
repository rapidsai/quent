// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { SelectedOperatorData } from '@quent/hooks';
import type { QuantitySpec } from '@quent/utils';
import { DataText } from '../ui/data-text';
import { StatisticField, StatisticFields } from './StatisticFields';

export const OperatorStatFields = ({
  operator,
  quantitySpecs,
}: {
  operator: SelectedOperatorData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) => (
  <>
    <div className="mb-2">
      <StatisticField name="ID">
        <DataText className="block truncate" title={operator.nodeId}>
          {operator.nodeId}
        </DataText>
      </StatisticField>
    </div>
    {operator.attributes?.length ? (
      <section className="mb-3">
        <h3 className="mb-1 text-xs font-semibold">Attributes</h3>
        <StatisticFields statistics={operator.attributes} quantitySpecs={quantitySpecs} />
      </section>
    ) : null}
    <section>
      <h3 className="mb-1 text-xs font-semibold">Statistics</h3>
      <StatisticFields statistics={operator.statistics} quantitySpecs={quantitySpecs} />
    </section>
  </>
);

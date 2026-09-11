// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  formatStatWithQuantity,
  type InspectedInformationItem,
  type InspectedOperatorData,
  type QuantitySpec,
  type StatValue,
} from '@quent/utils';
import { DataText } from '../ui/data-text';

function StatisticValue({
  name,
  value,
  quantity,
  quantitySpecs,
}: {
  name: string;
  value: StatValue;
  quantity?: string;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) {
  if (Array.isArray(value)) {
    return (
      <div className="ml-2 flex flex-col gap-0.5">
        {value.map((item, index) => (
          <DataText key={index} className="text-muted-foreground whitespace-pre-line">
            {String(item)}
          </DataText>
        ))}
      </div>
    );
  }

  return (
    <DataText className="text-muted-foreground ml-1">
      {typeof value === 'number' || typeof value === 'bigint'
        ? formatStatWithQuantity(
            value,
            name,
            quantity && quantitySpecs ? quantitySpecs[quantity] : undefined
          )
        : value == null
          ? '—'
          : String(value)}
    </DataText>
  );
}

function StatisticRows({
  statistics,
  quantitySpecs,
}: {
  statistics: readonly InspectedInformationItem[];
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) {
  return statistics.map(({ key, value, quantity }, index) => (
    <div key={`${key}-${index}`} className="text-xs flex items-start justify-between gap-2">
      <DataText className="capitalize">{key.replace(/_/g, ' ')}:</DataText>
      <StatisticValue name={key} value={value} quantity={quantity} quantitySpecs={quantitySpecs} />
    </div>
  ));
}

export const OperatorStatFields = ({
  operator,
  quantitySpecs,
}: {
  operator: InspectedOperatorData;
  quantitySpecs?: { [key: string]: QuantitySpec | undefined };
}) => {
  const information = operator.information;

  return (
    <>
      <div className="text-xs flex items-center justify-between">
        <DataText className="capitalize">ID:</DataText>
        <DataText className="text-muted-foreground ml-1 truncate">{operator.nodeId}</DataText>
      </div>
      {information ? (
        information.map((group, index) => (
          <section key={`${group.heading}-${index}`} className="mt-1">
            <DataText className="text-xs font-medium">{group.heading}</DataText>
            <StatisticRows statistics={group.items} quantitySpecs={quantitySpecs} />
          </section>
        ))
      ) : (
        <StatisticRows statistics={operator.statistics} quantitySpecs={quantitySpecs} />
      )}
      {(operator.portRelations?.length ?? 0) > 0 && (
        <section className="mt-1">
          <DataText className="text-xs font-medium">Port relations</DataText>
          {operator.portRelations?.map((relation, index) => (
            <div
              key={`${relation.portId}-${relation.role}-${index}`}
              className="text-xs flex gap-2"
            >
              <DataText className="capitalize">{relation.role}:</DataText>
              <DataText className="text-muted-foreground truncate">{relation.portId}</DataText>
            </div>
          ))}
        </section>
      )}
      {operator.observations?.map((observation, index) => (
        <section key={`${observation.kind}-${observation.timeSeconds}-${index}`} className="mt-1">
          <div className="text-xs flex items-center justify-between gap-2">
            <DataText className="font-medium">{observation.kind}</DataText>
            <DataText className="text-muted-foreground">
              {observation.timeSeconds.toFixed(6)} s
            </DataText>
          </div>
          <StatisticRows statistics={observation.attributes} />
          {observation.portRelations.map((relation, relationIndex) => (
            <div
              key={`${relation.portId}-${relation.role}-${relationIndex}`}
              className="text-xs flex gap-2"
            >
              <DataText className="capitalize">{relation.role} port:</DataText>
              <DataText className="text-muted-foreground truncate">{relation.portId}</DataText>
            </div>
          ))}
        </section>
      ))}
    </>
  );
};

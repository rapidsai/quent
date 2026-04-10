// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo } from 'react';
import carPriceCsv from '@/assets/car_price_dataset.csv?raw';
import { PivotTableToolbar } from './PivotTableToolbar';
import { StatGroupTable } from './StatGroupTable';
import type { StatGroupTableSchema } from './types';
import { getSchemaStatNames } from './utils';
import { useStatGroupTableControls } from './useStatGroupTableControls';

interface CarPriceRecord {
  carId: string;
  brand: string;
  modelYear: number;
  engineSize: number;
  fuelType: string;
  transmission: string;
  mileage: number;
  doors: number;
  ownerCount: number;
  horsepower: number;
  price: number;
}

type IndexKey = 'partition' | 'scope' | 'parent_item_type' | 'parent_item' | 'item_type' | 'item';

const INDEX_ORDER: IndexKey[] = [
  'partition',
  'scope',
  'parent_item_type',
  'parent_item',
  'item_type',
  'item',
];

const DEFAULT_ENABLED: Record<IndexKey, boolean> = {
  partition: true,
  scope: false,
  parent_item_type: true,
  parent_item: false,
  item_type: false,
  item: true,
};

function parseCarPriceCsv(csv: string): CarPriceRecord[] {
  const lines = csv
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter(Boolean);
  if (lines.length < 2) return [];

  const records: CarPriceRecord[] = [];
  for (const line of lines.slice(1)) {
    const cells = line.split(',');
    if (cells.length !== 11) continue;

    const [
      carId,
      brand,
      modelYear,
      engineSize,
      fuelType,
      transmission,
      mileage,
      doors,
      ownerCount,
      horsepower,
      price,
    ] = cells;

    records.push({
      carId,
      brand,
      modelYear: Number(modelYear),
      engineSize: Number(engineSize),
      fuelType,
      transmission,
      mileage: Number(mileage),
      doors: Number(doors),
      ownerCount: Number(ownerCount),
      horsepower: Number(horsepower),
      price: Number(price),
    });
  }

  return records.filter(record =>
    [
      record.modelYear,
      record.engineSize,
      record.mileage,
      record.doors,
      record.ownerCount,
      record.horsepower,
      record.price,
    ].every(Number.isFinite)
  );
}

const CAR_SCHEMA: StatGroupTableSchema<CarPriceRecord> = {
  groups: {
    partition: {
      id: row => row.brand,
    },
    scope: {
      id: row => row.fuelType,
    },
    parent_item_type: {
      id: row => row.transmission,
    },
    parent_item: {
      id: row => String(row.modelYear),
    },
    item_type: {
      id: row => `${row.doors}-door`,
    },
    item: {
      id: row => `car-${row.carId}`,
      label: row => `${row.brand} #${row.carId}`,
    },
  },
  itemId: row => `car-${row.carId}`,
  scopeId: row => row.fuelType,
  itemType: row => `${row.doors}-door`,
  stats: row => ({
    price_usd: row.price,
    horsepower: row.horsepower,
    mileage_mi: row.mileage,
    engine_size_l: row.engineSize,
    owner_count: row.ownerCount,
    model_year: row.modelYear,
    door_count: row.doors,
  }),
};

export function CarPriceStatGroupTablePoc() {
  const records = useMemo(() => parseCarPriceCsv(carPriceCsv), []);
  const allStatNames = useMemo(() => getSchemaStatNames(records, CAR_SCHEMA), [records]);
  const {
    aggMode,
    setAggMode,
    selectedStats,
    orderedStatNames,
    visibleStats,
    visibleIndexOrder,
    activeIndexKeys,
    isAggregating,
    enabledIndices,
    handleToggleIndex,
    handleReorderIndex,
    handleToggleStat,
    handleSelectAllStats,
    handleSelectNoStats,
  } = useStatGroupTableControls<IndexKey>({
    baseIndexOrder: INDEX_ORDER,
    defaultEnabled: DEFAULT_ENABLED,
    allStatNames,
    defaultStatSelector: stats =>
      ['price_usd', 'horsepower', 'mileage_mi', 'engine_size_l'].filter(stat =>
        stats.includes(stat)
      ),
  });

  const indexLabels: Record<string, React.ReactNode> = useMemo(
    () => ({
      partition: 'Brand',
      scope: 'Fuel Type',
      parent_item_type: 'Transmission',
      parent_item: 'Model Year',
      item_type: 'Door Layout',
      item: 'Car',
    }),
    []
  );

  const indexConfig = useMemo(
    () =>
      visibleIndexOrder.map(indexKey => ({
        key: indexKey,
        label: indexLabels[indexKey],
        enabled: enabledIndices[indexKey],
      })),
    [visibleIndexOrder, indexLabels, enabledIndices]
  );

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 border-b border-border bg-card px-3 py-2">
        <p className="text-sm text-muted-foreground">
          Car price dataset POC ({records.length.toLocaleString()} cars). Drag group chips, toggle
          columns, and click headers to sort.
        </p>
      </div>
      <div className="shrink-0 border-b border-border bg-card">
        <PivotTableToolbar
          indexConfig={indexConfig}
          isAggregating={isAggregating}
          aggMode={aggMode}
          orderedStats={orderedStatNames}
          selectedStats={selectedStats}
          onToggleIndex={handleToggleIndex}
          onReorderIndex={handleReorderIndex}
          onSetAggMode={setAggMode}
          onToggleStat={handleToggleStat}
          onSelectAllStats={handleSelectAllStats}
          onSelectNoStats={handleSelectNoStats}
        />
      </div>
      <div className="min-h-0 flex-1">
        <StatGroupTable
          rows={records}
          schema={CAR_SCHEMA}
          activeIndices={activeIndexKeys}
          visibleStats={visibleStats}
          isAggregating={isAggregating}
          aggMode={aggMode}
          indexLabels={indexLabels}
          virtualization={{
            enabled: true,
            estimateRowHeight: 34,
            overscan: 10,
          }}
        />
      </div>
    </div>
  );
}

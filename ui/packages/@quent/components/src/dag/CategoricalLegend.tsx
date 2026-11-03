// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { statisticFieldLabel, type StatisticField } from '@quent/utils';
import { CategoricalLegend as CategoricalLegendView } from '../ui/legend/CategoricalLegend';

interface CategoricalLegendProps {
  field: string | StatisticField;
  categoryMap: ReadonlyMap<string, string>;
  dimmedLabels?: ReadonlySet<string>;
  entrySuffixes?: ReadonlyMap<string, string>;
}

export function CategoricalLegend({
  field,
  categoryMap,
  dimmedLabels,
  entrySuffixes,
}: CategoricalLegendProps) {
  return (
    <CategoricalLegendView
      label={statisticFieldLabel(field)}
      categoryMap={categoryMap}
      dimmedLabels={dimmedLabels}
      entrySuffixes={entrySuffixes}
    />
  );
}

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { SelectField, type SelectFieldOption } from '../ui/select-field';
import {
  useSelectedColorField,
  useSelectedEdgeWidthField,
  useSelectedEdgeColorField,
  useSelectedNodeLabelField,
  useSelectedDagLayoutDirection,
  useNodeColorPalette,
  useEdgeColorPalette,
  useDataFlowEnabled,
  useSetDataFlowEnabled,
  useDataFlowMeta,
  useSelectedDataFlowMeasure,
  useSetSelectedDataFlowMeasure,
  resolveDataFlowMeasure,
} from '@quent/hooks';
import {
  NODE_LABEL_FIELD,
  DAG_LAYOUT_DIRECTION,
  type NodeLabelField,
  type DagLayoutDirection,
} from '@quent/utils';
import { Palette, Spline, Brush, Type, ArrowUpDown, Activity, Gauge } from 'lucide-react';
import { PalettePicker } from './PalettePicker';

interface DAGControlsProps {
  operatorStatFields: string[];
  portStatFields: string[];
  /** Whether dark mode is active. Passed explicitly to decouple from ThemeContext. */
  isDark: boolean;
}

const NODE_LABEL_OPTIONS: SelectFieldOption[] = [
  { value: NODE_LABEL_FIELD.NAME, label: 'Name' },
  { value: NODE_LABEL_FIELD.ID, label: 'ID' },
  { value: NODE_LABEL_FIELD.TYPE, label: 'Type' },
];

const LAYOUT_DIRECTION_OPTIONS: SelectFieldOption[] = [
  { value: DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP, label: 'Bottom to top' },
  { value: DAG_LAYOUT_DIRECTION.TOP_TO_BOTTOM, label: 'Top to bottom' },
];

/** DAG visual control toolbar: node color, edge width, edge color, node label field selectors. */
export const DAGControls = ({ operatorStatFields, portStatFields, isDark }: DAGControlsProps) => {
  const [colorField, setColorField] = useSelectedColorField();
  const [edgeWidthField, setEdgeWidthField] = useSelectedEdgeWidthField();
  const [edgeColorField, setEdgeColorField] = useSelectedEdgeColorField();
  const [nodeLabelField, setNodeLabelField] = useSelectedNodeLabelField();
  const [layoutDirection, setLayoutDirection] = useSelectedDagLayoutDirection();
  const [nodePalette, setNodePalette] = useNodeColorPalette();
  const [edgePalette, setEdgePalette] = useEdgeColorPalette();
  const dataFlowEnabled = useDataFlowEnabled();
  const setDataFlowEnabled = useSetDataFlowEnabled();
  const dataFlowMeta = useDataFlowMeta();
  const selectedDataFlowMeasure = useSelectedDataFlowMeasure();
  const setSelectedDataFlowMeasure = useSetSelectedDataFlowMeasure();

  const operatorOptions: SelectFieldOption[] = operatorStatFields.map(f => ({ value: f }));
  const portOptions: SelectFieldOption[] = portStatFields.map(f => ({ value: f }));

  const measureOptions: SelectFieldOption[] = (dataFlowMeta?.decl.measures ?? []).map(m => ({
    value: m.name,
    label: m.display_name,
  }));
  const effectiveMeasure = dataFlowMeta
    ? resolveDataFlowMeasure(selectedDataFlowMeasure, dataFlowMeta.decl)
    : null;

  return (
    <div className="bg-card">
      <div className="px-4 py-2">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Plan Controls
        </span>
      </div>
      <div className="px-4 pb-2 grid grid-cols-1 lg:grid-cols-2 gap-x-3 gap-y-1.5">
        <SelectField
          label="Node color"
          icon={Palette}
          options={operatorOptions}
          value={colorField ?? ''}
          onValueChange={setColorField}
          placeholder="None"
          triggerClassName="h-6 text-xs"
          trailingAdornment={
            <PalettePicker value={nodePalette} onValueChange={setNodePalette} isDark={isDark} />
          }
        />
        <SelectField
          label="Edge width"
          icon={Spline}
          options={portOptions}
          value={edgeWidthField ?? ''}
          onValueChange={setEdgeWidthField}
          placeholder="None"
          triggerClassName="h-6 text-xs"
        />
        <SelectField
          label="Edge color"
          icon={Brush}
          options={portOptions}
          value={edgeColorField ?? ''}
          onValueChange={setEdgeColorField}
          placeholder="None"
          triggerClassName="h-6 text-xs"
          trailingAdornment={
            <PalettePicker value={edgePalette} onValueChange={setEdgePalette} isDark={isDark} />
          }
        />
        <SelectField
          label="Node label"
          icon={Type}
          options={NODE_LABEL_OPTIONS}
          value={nodeLabelField}
          onValueChange={v => v && setNodeLabelField(v as NodeLabelField)}
          placeholder="Name"
          clearable={false}
          triggerClassName="h-6 text-xs"
        />
        <SelectField
          label="Layout direction"
          icon={ArrowUpDown}
          options={LAYOUT_DIRECTION_OPTIONS}
          value={layoutDirection}
          onValueChange={v => v && setLayoutDirection(v as DagLayoutDirection)}
          placeholder="Bottom to top"
          clearable={false}
          triggerClassName="h-6 text-xs"
        />
        {dataFlowMeta && (
          <label className="flex h-6 items-center gap-1.5 min-w-0 cursor-pointer select-none">
            <Activity className="h-3 w-3 shrink-0 text-muted-foreground" />
            <span className="text-xs text-muted-foreground shrink-0 whitespace-nowrap">
              Data flow
            </span>
            <input
              type="checkbox"
              checked={dataFlowEnabled}
              onChange={e => setDataFlowEnabled(e.target.checked)}
              className="h-3 w-3 rounded-sm accent-primary cursor-pointer"
            />
          </label>
        )}
        {dataFlowMeta && measureOptions.length > 1 && (
          <SelectField
            label="Flow measure"
            icon={Gauge}
            options={measureOptions}
            value={effectiveMeasure ?? ''}
            onValueChange={v => v && setSelectedDataFlowMeasure(v)}
            placeholder="Measure"
            clearable={false}
            triggerClassName="h-6 text-xs"
          />
        )}
      </div>
    </div>
  );
};

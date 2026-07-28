// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
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
  useDataFlowLabelMeasure,
  useSetDataFlowLabelMeasure,
  useSetDataFlowSelectedDimensions,
  resolveDataFlowMeasure,
} from '@quent/hooks';
import {
  cn,
  NODE_LABEL_FIELD,
  DAG_LAYOUT_DIRECTION,
  type NodeLabelField,
  type DagLayoutDirection,
} from '@quent/utils';
import {
  Palette,
  Spline,
  Brush,
  Type,
  ArrowUpDown,
  Activity,
  Gauge,
  Tags,
  Layers,
} from 'lucide-react';
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
  const dataFlowLabelMeasure = useDataFlowLabelMeasure();
  const setDataFlowLabelMeasure = useSetDataFlowLabelMeasure();
  const setDataFlowSelectedDimensions = useSetDataFlowSelectedDimensions();

  const operatorOptions: SelectFieldOption[] = operatorStatFields.map(f => ({ value: f }));
  const portOptions: SelectFieldOption[] = portStatFields.map(f => ({ value: f }));

  const measureOptions: SelectFieldOption[] = (dataFlowMeta?.decl.measures ?? []).map(m => ({
    value: m.name,
    label: m.display_name,
  }));
  const effectiveMeasure = dataFlowMeta
    ? resolveDataFlowMeasure(selectedDataFlowMeasure, dataFlowMeta.decl)
    : null;

  // Tier (dimension-key) selection chips. `dimensionSelection` on the meta
  // is the resolved selection (never empty); the LAST selected tier cannot
  // be unchecked — "nothing selected" is not a state, and stale selections
  // are reset to "all" by useDataFlowSync on a decl key-set change.
  const dimensionKeys = dataFlowMeta?.decl.dimension_keys ?? [];
  const dimensionSelection = dataFlowMeta?.dimensionSelection;
  const toggleDimension = (key: string) => {
    if (!dimensionSelection) return;
    const next = new Set(dimensionSelection);
    if (next.has(key)) {
      if (next.size <= 1) return;
      next.delete(key);
    } else {
      next.add(key);
    }
    // Normalize a full selection back to `null` (= all, survives new keys).
    setDataFlowSelectedDimensions(next.size === dimensionKeys.length ? null : next);
  };

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
        {dataFlowMeta && measureOptions.length > 1 && (
          <SelectField
            label="Bar labels"
            icon={Tags}
            options={measureOptions}
            value={dataFlowLabelMeasure ?? ''}
            onValueChange={setDataFlowLabelMeasure}
            placeholder="Follow measure"
            triggerClassName="h-6 text-xs"
          />
        )}
        {dataFlowMeta && dimensionSelection && dimensionKeys.length > 1 && (
          <div className="flex min-w-0 items-center gap-1.5 lg:col-span-2">
            <Layers className="h-3 w-3 shrink-0 text-muted-foreground" />
            <span className="text-xs text-muted-foreground shrink-0 whitespace-nowrap">
              {dataFlowMeta.decl.dimension_name}
            </span>
            <div className="flex min-w-0 flex-wrap items-center gap-1">
              {dimensionKeys.map(k => {
                const checked = dimensionSelection.has(k.key);
                const isLastChecked = checked && dimensionSelection.size <= 1;
                return (
                  <button
                    key={k.key}
                    type="button"
                    data-testid="flow-tier-toggle"
                    aria-pressed={checked}
                    disabled={isLastChecked}
                    title={isLastChecked ? 'At least one tier must stay selected' : k.display_name}
                    onClick={() => toggleDimension(k.key)}
                    className={cn(
                      'rounded-sm border px-1.5 py-0.5 text-[10px] leading-none whitespace-nowrap transition-colors cursor-pointer',
                      checked
                        ? 'border-primary/50 bg-primary/10 text-foreground'
                        : 'border-border text-muted-foreground opacity-60 hover:opacity-100',
                      isLastChecked && 'cursor-default'
                    )}
                  >
                    {k.display_name}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

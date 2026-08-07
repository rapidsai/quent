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
  NODE_LABEL_FIELD,
  DAG_LAYOUT_DIRECTION,
  type NodeLabelField,
  type DagLayoutDirection,
} from '@quent/utils';
import { Palette, Spline, Brush, Type, ArrowUpDown, Gauge, Tags, Layers } from 'lucide-react';
import { PalettePicker } from './PalettePicker';
import { ControlField, ControlGrid, ControlSection } from '../ui/control-grid';
import { RequiredMultiSelectField } from '../ui/required-multi-select-field';

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

  // The resolved selection is never empty; a full selection normalizes to "all".
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
    <div className="space-y-2 bg-card p-2">
      <ControlSection title="Plan controls">
        <ControlGrid columns={2} minColumnWidth="12rem">
          <ControlField
            label="Node color"
            icon={Palette}
            trailingAdornment={
              <PalettePicker value={nodePalette} onValueChange={setNodePalette} isDark={isDark} />
            }
          >
            <SelectField
              ariaLabel="Node color"
              options={operatorOptions}
              value={colorField ?? ''}
              onValueChange={setColorField}
              placeholder="None"
              triggerClassName="h-6 text-xs"
            />
          </ControlField>
          <ControlField label="Edge width" icon={Spline}>
            <SelectField
              ariaLabel="Edge width"
              options={portOptions}
              value={edgeWidthField ?? ''}
              onValueChange={setEdgeWidthField}
              placeholder="None"
              triggerClassName="h-6 text-xs"
            />
          </ControlField>
          <ControlField
            label="Edge color"
            icon={Brush}
            trailingAdornment={
              <PalettePicker value={edgePalette} onValueChange={setEdgePalette} isDark={isDark} />
            }
          >
            <SelectField
              ariaLabel="Edge color"
              options={portOptions}
              value={edgeColorField ?? ''}
              onValueChange={setEdgeColorField}
              placeholder="None"
              triggerClassName="h-6 text-xs"
            />
          </ControlField>
          <ControlField label="Node label" icon={Type}>
            <SelectField
              ariaLabel="Node label"
              options={NODE_LABEL_OPTIONS}
              value={nodeLabelField}
              onValueChange={v => v && setNodeLabelField(v as NodeLabelField)}
              placeholder="Name"
              clearable={false}
              triggerClassName="h-6 text-xs"
            />
          </ControlField>
          <ControlField label="Layout direction" icon={ArrowUpDown}>
            <SelectField
              ariaLabel="Layout direction"
              options={LAYOUT_DIRECTION_OPTIONS}
              value={layoutDirection}
              onValueChange={v => v && setLayoutDirection(v as DagLayoutDirection)}
              placeholder="Bottom to top"
              clearable={false}
              triggerClassName="h-6 text-xs"
            />
          </ControlField>
        </ControlGrid>
      </ControlSection>

      {dataFlowMeta && (
        <ControlSection
          title="Data flow"
          action={
            <label className="flex cursor-pointer select-none items-center gap-1.5 text-xs text-muted-foreground">
              <input
                type="checkbox"
                checked={dataFlowEnabled}
                onChange={e => setDataFlowEnabled(e.target.checked)}
                className="size-3 cursor-pointer accent-primary"
              />
              Enabled
            </label>
          }
        >
          <fieldset
            disabled={!dataFlowEnabled}
            className="m-0 min-w-0 border-0 p-0 disabled:pointer-events-none disabled:opacity-50"
          >
            <legend className="sr-only">Data flow settings</legend>
            <ControlGrid columns={2} minColumnWidth="12rem">
              {measureOptions.length > 1 && (
                <ControlField label="Flow measure" icon={Gauge}>
                  <SelectField
                    ariaLabel="Flow measure"
                    options={measureOptions}
                    value={effectiveMeasure ?? ''}
                    onValueChange={v => v && setSelectedDataFlowMeasure(v)}
                    placeholder="Measure"
                    clearable={false}
                    triggerClassName="h-6 text-xs"
                  />
                </ControlField>
              )}
              {measureOptions.length > 1 && (
                <ControlField label="Bar labels" icon={Tags}>
                  <SelectField
                    ariaLabel="Bar labels"
                    options={measureOptions}
                    value={dataFlowLabelMeasure ?? ''}
                    onValueChange={setDataFlowLabelMeasure}
                    placeholder="Follow measure"
                    triggerClassName="h-6 text-xs"
                  />
                </ControlField>
              )}
              {dimensionSelection && dimensionKeys.length > 1 && (
                <ControlField
                  label={dataFlowMeta.decl.dimension_name}
                  icon={Layers}
                  align="start"
                  className="col-span-full"
                >
                  <RequiredMultiSelectField
                    label={dataFlowMeta.decl.dimension_name}
                    options={dimensionKeys.map(option => ({
                      value: option.key,
                      label: option.display_name,
                    }))}
                    selected={dimensionSelection}
                    onToggle={toggleDimension}
                    optionTestId="flow-tier-toggle"
                  />
                </ControlField>
              )}
            </ControlGrid>
          </fieldset>
        </ControlSection>
      )}
    </div>
  );
};

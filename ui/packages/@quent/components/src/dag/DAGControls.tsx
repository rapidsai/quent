// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';
import { SelectField, type SelectFieldOption } from '../ui/select-field';
import {
  useSelectedColorField,
  useSelectedEdgeWidthField,
  useSelectedEdgeColorField,
  useSelectedNodeLabelField,
} from '@quent/hooks';
import { NODE_LABEL_FIELD, type NodeLabelField } from '@quent/utils';
import { Palette, Spline, Brush, ChevronDown, Type } from 'lucide-react';
import { DAGSettingsPopover } from './DAGSettingsPopover';
import { useState } from 'react';

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

/** DAG visual control toolbar: node color, edge width, edge color, node label field selectors. */
export const DAGControls = ({ operatorStatFields, portStatFields, isDark }: DAGControlsProps) => {
  const [colorField, setColorField] = useSelectedColorField();
  const [edgeWidthField, setEdgeWidthField] = useSelectedEdgeWidthField();
  const [edgeColorField, setEdgeColorField] = useSelectedEdgeColorField();
  const [nodeLabelField, setNodeLabelField] = useSelectedNodeLabelField();
  const [open, setOpen] = useState(true);

  const operatorOptions: SelectFieldOption[] = operatorStatFields.map(f => ({ value: f }));
  const portOptions: SelectFieldOption[] = portStatFields.map(f => ({ value: f }));

  return (
    <Collapsible open={open} onOpenChange={setOpen} className="border-b bg-card">
      <div className="flex items-center justify-between px-4 py-2">
        <CollapsibleTrigger className="flex items-center gap-2 group">
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            Plan Controls
          </span>
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground transition-transform duration-200 cursor-pointer group-data-[state=open]:rotate-180" />
        </CollapsibleTrigger>
        <DAGSettingsPopover isDark={isDark} />
      </div>
      <CollapsibleContent className="px-4 pb-2 grid grid-cols-1 lg:grid-cols-2 gap-x-3 gap-y-1.5">
        <SelectField
          label="Node color"
          icon={Palette}
          options={operatorOptions}
          value={colorField ?? ''}
          onValueChange={setColorField}
          placeholder="None"
          triggerClassName="h-6 text-xs"
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
      </CollapsibleContent>
    </Collapsible>
  );
};

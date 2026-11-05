// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef } from 'react';

export interface SubRowVisibilityControl {
  id: string;
  label: string;
  ids: readonly string[];
}

function VisibilityCheckbox({
  control,
  expandedIds,
  onChange,
}: {
  control: SubRowVisibilityControl;
  expandedIds: ReadonlySet<string>;
  onChange: (ids: readonly string[], expanded: boolean) => void;
}) {
  const checkboxRef = useRef<HTMLInputElement>(null);
  const expandedCount = control.ids.filter(id => expandedIds.has(id)).length;
  const allExpanded = expandedCount === control.ids.length;
  const partiallyExpanded = expandedCount > 0 && !allExpanded;

  useEffect(() => {
    if (checkboxRef.current) {
      checkboxRef.current.indeterminate = partiallyExpanded;
    }
  }, [partiallyExpanded]);

  return (
    <label className="flex cursor-pointer items-center gap-1 whitespace-nowrap text-xs text-muted-foreground">
      <input
        ref={checkboxRef}
        aria-checked={partiallyExpanded ? 'mixed' : allExpanded}
        aria-label={`${control.label} charts`}
        checked={allExpanded}
        className="h-3 w-3 accent-primary"
        onChange={event => onChange(control.ids, event.target.checked)}
        type="checkbox"
      />
      <span>
        {control.label} {expandedCount}/{control.ids.length}
      </span>
    </label>
  );
}

export function SubRowVisibilityControls({
  controls,
  expandedIds,
  onChange,
}: {
  controls: readonly SubRowVisibilityControl[];
  expandedIds: ReadonlySet<string>;
  onChange: (ids: readonly string[], expanded: boolean) => void;
}) {
  const availableControls = controls.filter(control => control.ids.length > 0);
  if (availableControls.length === 0) {
    return null;
  }

  return (
    <div className="flex items-center gap-3 border-l border-border pl-3">
      {availableControls.map(control => (
        <VisibilityCheckbox
          key={control.id}
          control={control}
          expandedIds={expandedIds}
          onChange={onChange}
        />
      ))}
    </div>
  );
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn } from '@quent/utils';

export interface RequiredMultiSelectOption {
  value: string;
  label: string;
}

export interface RequiredMultiSelectFieldProps {
  label: string;
  options: RequiredMultiSelectOption[];
  selected: ReadonlySet<string>;
  onToggle: (value: string) => void;
  helperText?: string;
  optionTestId?: string;
  className?: string;
}

/** Checkbox group that requires at least one selected option. */
export function RequiredMultiSelectField({
  label,
  options,
  selected,
  onToggle,
  helperText = 'Select one or more. At least one option is required.',
  optionTestId,
  className,
}: RequiredMultiSelectFieldProps) {
  return (
    <fieldset className={className}>
      <legend className="sr-only">{label}</legend>
      <div className="flex flex-wrap gap-x-3 gap-y-1.5">
        {options.map(option => {
          const checked = selected.has(option.value);
          const isLastChecked = checked && selected.size <= 1;
          return (
            <label
              key={option.value}
              className={cn(
                'flex h-6 cursor-pointer items-center gap-1.5 rounded-sm border px-2 text-xs transition-colors',
                checked
                  ? 'border-primary/50 bg-primary/10 text-foreground'
                  : 'border-border bg-background text-muted-foreground hover:bg-accent',
                isLastChecked && 'cursor-default'
              )}
            >
              <input
                type="checkbox"
                data-testid={optionTestId}
                checked={checked}
                disabled={isLastChecked}
                onChange={() => onToggle(option.value)}
                className="size-3 cursor-pointer accent-primary disabled:cursor-default"
              />
              <span>{option.label}</span>
            </label>
          );
        })}
      </div>
      <p className="mt-1 text-[10px] leading-tight text-muted-foreground">{helperText}</p>
    </fieldset>
  );
}

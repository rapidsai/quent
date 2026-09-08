// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn } from '@quent/utils';
import { Input } from './input';
import { Slider } from './slider';
import {
  clamp,
  niceSliderStep,
  parseOptionalNumber,
  resolveSliderValue,
} from '../lib/sliderField.utils';

export interface SliderFieldProps {
  label: string;
  value: string;
  className?: string;
  min: number;
  max: number;
  invalid?: boolean;
  /** id of an element (e.g. an error message) to reference via aria-describedby when invalid. */
  errorMessageId?: string;
  onChange: (value: string) => void;
}

/** A single-thumb slider paired with a precise number input, bounded to [min, max]. */
export function SliderField({
  label,
  value,
  className,
  min,
  max,
  invalid,
  errorMessageId,
  onChange,
}: SliderFieldProps) {
  const step = niceSliderStep(max - min);
  const sliderValue = clamp(parseOptionalNumber(value) ?? min, min, max);

  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      <span className="text-xs text-muted-foreground">{label}</span>
      <Slider
        aria-label={`${label} slider`}
        min={min}
        max={max}
        step={step}
        value={sliderValue}
        onValueChange={next => onChange(resolveSliderValue(next as number, step, min, max))}
        className="px-0.5"
      />
      <Input
        type="number"
        min={0}
        step="any"
        className="mt-0.5 h-8 rounded-sm px-2 text-xs md:text-xs"
        value={value}
        aria-label={label}
        aria-invalid={invalid}
        aria-describedby={invalid ? errorMessageId : undefined}
        onChange={event => onChange(event.target.value)}
      />
    </div>
  );
}

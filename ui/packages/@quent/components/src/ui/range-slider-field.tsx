// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { cn } from '@quent/utils';
import { Input } from './input';
import { Slider } from './slider';
import { clamp, formatStep, niceSliderStep, parseOptionalNumber } from '../lib/sliderField.utils';

export interface RangeSliderFieldProps {
  label: string;
  /** Accessible label for the start number input (visible label stays the group `label`). */
  startLabel: string;
  /** Accessible label for the end number input (visible label stays the group `label`). */
  endLabel: string;
  className?: string;
  min: number;
  max: number;
  startValue: string;
  endValue: string;
  invalidStart?: boolean;
  invalidEnd?: boolean;
  /** id of an element (e.g. an error message) to reference via aria-describedby when invalid. */
  errorMessageId?: string;
  onStartChange: (value: string) => void;
  onEndChange: (value: string) => void;
}

/** A dual-thumb range slider paired with start/end number inputs, bounded to [min, max]. */
export function RangeSliderField({
  label,
  startLabel,
  endLabel,
  className,
  min,
  max,
  startValue,
  endValue,
  invalidStart,
  invalidEnd,
  errorMessageId,
  onStartChange,
  onEndChange,
}: RangeSliderFieldProps) {
  const step = niceSliderStep(max - min);
  const sliderValue: [number, number] = [
    clamp(parseOptionalNumber(startValue) ?? min, min, max),
    clamp(parseOptionalNumber(endValue) ?? max, min, max),
  ];

  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      <span className="text-xs text-muted-foreground">{label}</span>
      <Slider
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={sliderValue}
        onValueChange={next => {
          const [start, end] = next as number[];
          // Only the dragged thumb's value is reformatted from the slider — leave the other
          // field's string untouched so a precisely-typed value isn't rounded to the slider step.
          if (start !== sliderValue[0]) {
            onStartChange(formatStep(start!, step));
          }
          if (end !== sliderValue[1]) {
            onEndChange(formatStep(end!, step));
          }
        }}
        className="px-0.5"
      />
      <div className="mt-0.5 flex items-center gap-1">
        <Input
          type="number"
          min={0}
          step="any"
          className="h-8 min-w-0 flex-1 rounded-sm px-2 text-xs md:text-xs"
          value={startValue}
          aria-label={startLabel}
          aria-invalid={invalidStart}
          aria-describedby={invalidStart ? errorMessageId : undefined}
          onChange={event => onStartChange(event.target.value)}
        />
        <span className="text-muted-foreground">–</span>
        <Input
          type="number"
          min={0}
          step="any"
          className="h-8 min-w-0 flex-1 rounded-sm px-2 text-xs md:text-xs"
          value={endValue}
          aria-label={endLabel}
          aria-invalid={invalidEnd}
          aria-describedby={invalidEnd ? errorMessageId : undefined}
          onChange={event => onEndChange(event.target.value)}
        />
      </div>
    </div>
  );
}

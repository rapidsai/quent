// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { CSSProperties, ElementType, HTMLAttributes, ReactNode } from 'react';
import { cn } from '@quent/utils';

export interface ControlSectionProps {
  title: string;
  description?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  contentClassName?: string;
}

export interface ControlGridProps extends HTMLAttributes<HTMLDivElement> {
  /** Maximum number of columns. */
  columns?: number;
  /** Wrap into fewer columns when one would be narrower than this value. */
  minColumnWidth?: CSSProperties['width'];
}

export interface ControlFieldProps {
  label: string;
  icon?: ElementType;
  trailingAdornment?: ReactNode;
  className?: string;
  align?: 'center' | 'start';
  labelWidth?: CSSProperties['width'];
  adornmentWidth?: CSSProperties['width'];
  children: ReactNode;
}

function toCssSize(value: CSSProperties['width']): string {
  return typeof value === 'number' ? `${value}px` : (value ?? '0');
}

/** Visual section for a related group of controls. */
export function ControlSection({
  title,
  description,
  action,
  children,
  className,
  contentClassName,
}: ControlSectionProps) {
  return (
    <section className={cn('border-b border-border last:border-b-0 py-1', className)}>
      <header className="flex min-h-7 items-center justify-between gap-3 px-3 py-1">
        <div className="min-w-0">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {title}
          </h3>
          {description && (
            <p className="mt-0.5 text-[10px] leading-tight text-muted-foreground">{description}</p>
          )}
        </div>
        {action}
      </header>
      <div className={cn('p-3', contentClassName)}>{children}</div>
    </section>
  );
}

/** Grid with an explicit, caller-configured column count. */
export function ControlGrid({
  columns = 1,
  minColumnWidth,
  className,
  style,
  children,
  ...props
}: ControlGridProps) {
  const columnCount = Math.max(1, Math.floor(columns));
  const columnTemplate = minColumnWidth
    ? `repeat(auto-fit, minmax(min(100%, max(${toCssSize(minColumnWidth)}, calc((100% - ${(columnCount - 1) * 1.5}rem) / ${columnCount}))), 1fr))`
    : `repeat(${columnCount}, minmax(0, 1fr))`;
  return (
    <div
      className={cn('grid gap-x-6 gap-y-2', className)}
      style={{
        ...style,
        gridTemplateColumns: columnTemplate,
      }}
      {...props}
    >
      {children}
    </div>
  );
}

/** Aligns a label, control, and optional adornment within a control grid. */
export function ControlField({
  label,
  icon: Icon,
  trailingAdornment,
  className,
  align = 'center',
  labelWidth = 'fit-content(7rem)',
  adornmentWidth = '1.5rem',
  children,
}: ControlFieldProps) {
  const hasTrailingAdornment = Boolean(trailingAdornment);

  return (
    <div
      className={cn(
        'grid min-w-0 gap-2',
        align === 'center' ? 'items-center' : 'items-start',
        className
      )}
      style={{
        gridTemplateColumns: `${toCssSize(labelWidth)} minmax(0, 1fr)${
          hasTrailingAdornment ? ` ${toCssSize(adornmentWidth)}` : ''
        }`,
      }}
    >
      <div className={cn('flex min-w-0 items-center gap-1.5', align === 'start' && 'pt-1')}>
        {Icon && <Icon className="size-3 shrink-0 text-muted-foreground" />}
        <span className="truncate text-xs text-muted-foreground">{label}</span>
      </div>
      <div className="min-w-0">{children}</div>
      {hasTrailingAdornment && (
        <div className="flex size-6 items-center justify-center">{trailingAdornment}</div>
      )}
    </div>
  );
}

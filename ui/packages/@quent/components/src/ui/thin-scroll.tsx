// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import * as React from 'react';

import { cn } from '@quent/utils';

/**
 * Tailwind class string that styles the native scrollbar of an element to
 * match Quent's thin, themed look (1.5px rounded thumb on `bg-border`,
 * transparent track). Apply to any element that already has an `overflow-*`
 * utility, or use the `<ThinScroll>` wrapper for the common case.
 */
export const thinScrollbarClass =
  '[&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar]:h-1.5 ' +
  '[&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-border ' +
  '[&::-webkit-scrollbar-track]:bg-transparent ' +
  '[scrollbar-width:thin] [scrollbar-color:hsl(var(--border))_transparent]';

type ThinScrollOrientation = 'vertical' | 'horizontal' | 'both';

const OVERFLOW_BY_ORIENTATION: Record<ThinScrollOrientation, string> = {
  vertical: 'overflow-y-auto overflow-x-hidden',
  horizontal: 'overflow-x-auto overflow-y-hidden',
  both: 'overflow-auto',
};

export interface ThinScrollProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Which axis scrolls. Defaults to `vertical`. */
  orientation?: ThinScrollOrientation;
}

/**
 * Scroll container with Quent's thin, themed native scrollbar. Use when the
 * extra DOM/JS overhead of `ScrollArea` (Radix) isn't needed — e.g. wrapping
 * an ECharts canvas or any element that needs a real native scroll viewport.
 */
export const ThinScroll = React.forwardRef<HTMLDivElement, ThinScrollProps>(
  ({ orientation = 'vertical', className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(OVERFLOW_BY_ORIENTATION[orientation], thinScrollbarClass, className)}
      {...props}
    />
  )
);
ThinScroll.displayName = 'ThinScroll';

/**
 * Tailwind class string that hides the native scrollbar on every browser
 * while leaving the element scrollable. Pair with an `overflow-*` utility
 * or use the `<HiddenScroll>` wrapper.
 */
export const hiddenScrollbarClass =
  '[&::-webkit-scrollbar]:w-0 [&::-webkit-scrollbar]:h-0 ' +
  '[scrollbar-width:none] [-ms-overflow-style:none]';

export type HiddenScrollProps = ThinScrollProps;

/**
 * Scroll container that scrolls but renders no visible scrollbar. Useful
 * when scroll affordance is implicit (e.g. a Gantt that scrolls vertically
 * inside a fixed-height parent and doesn't need a separate visual indicator).
 */
export const HiddenScroll = React.forwardRef<HTMLDivElement, HiddenScrollProps>(
  ({ orientation = 'vertical', className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(OVERFLOW_BY_ORIENTATION[orientation], hiddenScrollbarClass, className)}
      {...props}
    />
  )
);
HiddenScroll.displayName = 'HiddenScroll';

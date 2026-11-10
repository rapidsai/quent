// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DeepLinkTab } from '@/features/deep-link/deepLink.schema';

const TAB_LABELS: Record<DeepLinkTab, string> = {
  timeline: 'Timeline',
  operators: 'Operators',
  entities: 'Entities',
};

const DEFAULT_NAME_FORMAT = new Intl.DateTimeFormat(undefined, {
  month: 'short',
  day: 'numeric',
  hour: 'numeric',
  minute: '2-digit',
});

export function defaultSavedViewName(
  tab: DeepLinkTab,
  now: Date,
  existingNames: readonly string[]
): string {
  const base = `${TAB_LABELS[tab]} · ${DEFAULT_NAME_FORMAT.format(now)}`;
  const names = new Set(existingNames.map(name => name.toLocaleLowerCase()));
  if (!names.has(base.toLocaleLowerCase())) {
    return base;
  }
  for (let suffix = 2; ; suffix += 1) {
    const candidate = `${base} (${suffix})`;
    if (!names.has(candidate.toLocaleLowerCase())) {
      return candidate;
    }
  }
}

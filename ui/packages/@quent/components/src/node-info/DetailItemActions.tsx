// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { X } from 'lucide-react';

interface DetailItemActionsProps {
  label: string;
  title?: string;
  onRemove: () => void;
}

const actionClass =
  'inline-flex size-6 cursor-pointer items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-muted-foreground';

export function DetailItemActions({ label, title, onRemove }: DetailItemActionsProps) {
  return (
    <div className="flex shrink-0 items-center gap-0.5" onClick={event => event.stopPropagation()}>
      <button
        type="button"
        aria-label={`Remove ${label} from details`}
        title={title ?? 'Remove from details'}
        className={actionClass}
        onClick={onRemove}
      >
        <X aria-hidden className="size-3.5" />
      </button>
    </div>
  );
}

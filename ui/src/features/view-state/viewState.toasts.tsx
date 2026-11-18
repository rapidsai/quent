// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { ChevronDown } from 'lucide-react';
import { toast } from '@quent/components';

export const MAX_INDIVIDUAL_VIEW_STATE_ERROR_TOASTS = 3;

interface ShowViewStateErrorToastsOptions {
  id: string;
  title: string;
  messages: readonly string[];
}

function ViewStateErrorDetails({ messages }: { messages: readonly string[] }) {
  return (
    <span
      role="region"
      aria-label="Deep link validation errors"
      tabIndex={0}
      className="block max-h-48 space-y-1 overflow-y-auto pr-2"
    >
      {messages.map((message, index) => (
        <span key={`${index}-${message}`} className="block">
          <span aria-hidden="true">• </span>
          {message}
        </span>
      ))}
    </span>
  );
}

function getErrorDetailsToggleProps(
  summaryId: string,
  messages: readonly string[],
  expanded: boolean
) {
  return {
    description: expanded ? <ViewStateErrorDetails messages={messages} /> : null,
    actionProps: {
      'aria-expanded': expanded,
      children: expanded ? (
        <>
          Hide details
          <ChevronDown className="size-3 transition-transform" />
        </>
      ) : (
        <>
          {messages.length} errors, click to expand
          <ChevronDown className="size-3 -rotate-90 transition-transform" />
        </>
      ),
      onClick: () => {
        toast.update(summaryId, getErrorDetailsToggleProps(summaryId, messages, !expanded));
      },
    },
  };
}

export function showViewStateErrorToasts({ id, title, messages }: ShowViewStateErrorToastsOptions) {
  if (messages.length > MAX_INDIVIDUAL_VIEW_STATE_ERROR_TOASTS) {
    const summaryId = `${id}-summary`;

    toast.add({
      id: summaryId,
      type: 'error',
      title,
      ...getErrorDetailsToggleProps(summaryId, messages, false),
      priority: 'high',
      timeout: 0,
    });
    return;
  }

  messages.forEach((description, index) => {
    toast.add({
      id: `${id}-${index}`,
      type: 'error',
      title,
      description,
      priority: 'high',
    });
  });
}

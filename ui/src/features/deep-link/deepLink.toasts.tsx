// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { ChevronDown } from 'lucide-react';
import { toast } from '@quent/components';

export const MAX_INDIVIDUAL_DEEP_LINK_ERROR_TOASTS = 3;

interface ShowDeepLinkErrorToastsOptions {
  id: string;
  title: string;
  messages: readonly string[];
}

function DeepLinkErrorDetails({ messages }: { messages: readonly string[] }) {
  return (
    <span className="block max-h-48 space-y-1 overflow-y-auto pr-2">
      {messages.map((message, index) => (
        <span key={`${index}-${message}`} className="block">
          <span aria-hidden="true">• </span>
          {message}
        </span>
      ))}
    </span>
  );
}

export function showDeepLinkErrorToasts({ id, title, messages }: ShowDeepLinkErrorToastsOptions) {
  if (messages.length > MAX_INDIVIDUAL_DEEP_LINK_ERROR_TOASTS) {
    const summaryId = `${id}-summary`;

    function showSummary() {
      toast.update(summaryId, {
        description: null,
        actionProps: {
          'aria-expanded': false,
          children: (
            <>
              {messages.length} errors, click to expand
              <ChevronDown className="size-3 -rotate-90 transition-transform" />
            </>
          ),
          onClick: showDetails,
        },
      });
    }

    function showDetails() {
      toast.update(summaryId, {
        description: <DeepLinkErrorDetails messages={messages} />,
        actionProps: {
          'aria-expanded': true,
          children: (
            <>
              Hide details
              <ChevronDown className="size-3 transition-transform" />
            </>
          ),
          onClick: showSummary,
        },
      });
    }

    toast.add({
      id: summaryId,
      type: 'error',
      title,
      description: null,
      actionProps: {
        'aria-expanded': false,
        children: (
          <>
            {messages.length} errors, click to expand
            <ChevronDown className="size-3 -rotate-90 transition-transform" />
          </>
        ),
        onClick: showDetails,
      },
      priority: 'high',
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

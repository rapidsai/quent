// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';
import { Button } from '@quent/components';
import type { FiniteStateMachine } from '@quent/utils';
import { EntityDetailPanel } from './entities-table/EntityDetailPanel';

interface EntityDetailDrawerProps {
  fsm: FiniteStateMachine | null;
  resourceLabel: (id: string) => string;
  operatorLabel: (id: string) => string;
  onClose: () => void;
}

export function EntityDetailDrawer({
  fsm,
  resourceLabel,
  operatorLabel,
  onClose,
}: EntityDetailDrawerProps) {
  useEffect(() => {
    if (!fsm) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [fsm, onClose]);

  return createPortal(
    <div
      role="complementary"
      aria-label="Entity details"
      className={`fixed right-0 top-0 z-50 flex h-full w-80 flex-col border-l bg-background shadow-xl transition-transform duration-200 ${
        fsm ? 'translate-x-0' : 'translate-x-full'
      }`}
    >
      <div className="flex shrink-0 items-center justify-between border-b bg-card px-3 py-2">
        <span className="text-sm font-medium">Entity details</span>
        <Button variant="ghost" size="icon" aria-label="Close" onClick={onClose}>
          <X className="size-4" />
        </Button>
      </div>
      <div className="min-h-0 flex-1">
        <EntityDetailPanel
          fsm={fsm}
          resourceLabel={resourceLabel}
          operatorLabel={operatorLabel}
        />
      </div>
    </div>,
    document.body
  );
}

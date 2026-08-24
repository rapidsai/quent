// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { X } from 'lucide-react';
import {
  Button,
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerPortal,
  DrawerTitle,
} from '@quent/components';
import type { EntityRef, FiniteStateMachine, QueryBundle } from '@quent/utils';
import { EntityDetailPanel } from './entities-table/EntityDetailPanel';

interface EntityDetailDrawerProps {
  fsm: FiniteStateMachine | null;
  resourceLabel: (id: string) => string;
  operatorLabel: (id: string) => string;
  onClose: () => void;
  stateColorFn?: (name: string) => string;
  queryBundle: QueryBundle<EntityRef>;
}

export function EntityDetailDrawer({
  fsm,
  resourceLabel,
  operatorLabel,
  onClose,
  stateColorFn,
  queryBundle,
}: EntityDetailDrawerProps) {
  return (
    <Drawer
      open={fsm !== null}
      onOpenChange={open => {
        if (!open) onClose();
      }}
      direction="right"
      modal={false}
      noBodyStyles
      shouldScaleBackground={false}
      handleOnly
    >
      <DrawerPortal>
        <DrawerContent
          onPointerDownOutside={event => {
            const target = event.detail.originalEvent.target;
            // Entity clicks on the long-entities Gantt already toggle the
            // selection via onEntitySelect; closing here first would clear
            // drawerFsm before that handler runs, breaking the toggle.
            if (target instanceof Element && target.closest('[data-long-entities-gantt]')) {
              return;
            }
            onClose();
          }}
          className="h-full w-80 shadow-xl sm:max-w-none"
        >
          <div className="flex shrink-0 items-center justify-between border-b bg-card px-3 py-2">
            <DrawerTitle className="text-sm">Entity details</DrawerTitle>
            <DrawerDescription className="sr-only">
              Details for the selected entity.
            </DrawerDescription>
            <DrawerClose asChild>
              <Button variant="ghost" size="icon" aria-label="Close">
                <X className="size-4" />
              </Button>
            </DrawerClose>
          </div>
          <div className="min-h-0 flex-1">
            <EntityDetailPanel
              fsm={fsm}
              resourceLabel={resourceLabel}
              operatorLabel={operatorLabel}
              stateColorFn={stateColorFn}
              queryBundle={queryBundle}
            />
          </div>
        </DrawerContent>
      </DrawerPortal>
    </Drawer>
  );
}

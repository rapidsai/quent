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
import { NvtxDetailPanel } from './nvtx-table/NvtxDetailPanel';

interface NvtxDetailDrawerProps {
  contextId: string | null;
  spanId: number | null;
  queryStartUnixNs: bigint;
  onClose: () => void;
  onSelectSpan: (spanId: number) => void;
}

export function NvtxDetailDrawer({
  contextId,
  spanId,
  queryStartUnixNs,
  onClose,
  onSelectSpan,
}: NvtxDetailDrawerProps) {
  return (
    <Drawer
      open={spanId !== null}
      onOpenChange={open => {
        if (!open) {
          onClose();
        }
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
            // Range clicks on the NVTX Gantt already toggle the selection via
            // onRangeSelect; closing here first would clear drawerSpanId
            // before that handler runs, breaking the toggle.
            if (target instanceof Element && target.closest('[data-nvtx-gantt]')) {
              return;
            }
            onClose();
          }}
          className="h-full w-80 shadow-xl sm:max-w-none"
        >
          <div className="flex shrink-0 items-center justify-between border-b bg-card px-3 py-2">
            <DrawerTitle className="text-sm">NVTX range details</DrawerTitle>
            <DrawerDescription className="sr-only">
              Details for the selected NVTX range.
            </DrawerDescription>
            <DrawerClose asChild>
              <Button variant="ghost" size="icon" aria-label="Close">
                <X className="size-4" />
              </Button>
            </DrawerClose>
          </div>
          <div className="min-h-0 flex-1">
            <NvtxDetailPanel
              contextId={contextId}
              spanId={spanId}
              queryStartUnixNs={queryStartUnixNs}
              onSelectSpan={onSelectSpan}
            />
          </div>
        </DrawerContent>
      </DrawerPortal>
    </Drawer>
  );
}

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { ResourceGroup } from '@quent/utils';
import { InlineSelector } from './InlineSelector';
import { DataText } from '../ui/data-text';

const FSM_ALL = 'All';

interface ResourceGroupRowProps {
  group: ResourceGroup;
  id: string;
  availableResourceTypes?: string[];
  selectedType?: string;
  onTypeChange?: (itemId: string, type: string) => void;
  availableFsmTypes?: string[];
  selectedFsmType?: string | null;
  onFsmChange?: (itemId: string, fsmType: string | null) => void;
  verbose?: boolean;
}

/** Group row showing resource group name and optional resource-type/FSM inline selectors. */
export const ResourceGroupRow = ({
  group,
  id,
  availableResourceTypes,
  selectedType,
  onTypeChange,
  availableFsmTypes,
  selectedFsmType,
  onFsmChange,
}: ResourceGroupRowProps): React.ReactNode => {
  const hasMultipleChildTypes = (availableResourceTypes?.length ?? 0) > 1;
  const fsmCount = availableFsmTypes?.length ?? 0;
  const hasOneFsm = fsmCount === 1;
  const hasMultipleFsms = fsmCount > 1;
  const fsmOptions = hasMultipleFsms ? [FSM_ALL, ...(availableFsmTypes ?? [])] : [];

  const showType = hasMultipleChildTypes && selectedType && onTypeChange && availableResourceTypes;
  const showFsmStatic = hasOneFsm;
  const showFsmSelector = hasMultipleFsms && onFsmChange && fsmOptions.length > 0;
  const hasMetadata = showType || showFsmStatic || showFsmSelector;

  return (
    <div className="pb-1">
      <DataText className="text-xs font-bold leading-none">{group.instance_name}</DataText>
      {hasMetadata && (
        <div className="flex flex-col gap-y-1">
          {showType && (
            <InlineSelector
              id={`${id}-resource-type`}
              label="Type"
              value={selectedType!}
              options={availableResourceTypes!}
              onChange={(_, value) => onTypeChange!(id, value)}
            />
          )}
          {showFsmStatic && (
            <span className="text-[11px] leading-none text-muted-foreground">
              FSM: <DataText className="text-foreground">{availableFsmTypes![0]}</DataText>
            </span>
          )}
          {showFsmSelector && (
            <InlineSelector
              id={`${id}-fsm`}
              label="FSM"
              value={selectedFsmType ?? FSM_ALL}
              options={fsmOptions}
              onChange={(_, value) => onFsmChange!(id, value === FSM_ALL ? null : value)}
            />
          )}
        </div>
      )}
    </div>
  );
};

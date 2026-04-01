// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Resource } from '~quent/types/Resource';
import { DataText } from '@/components/ui/data-text';

interface ResourceRowProps {
  resource: Resource;
}

export const ResourceRow = ({ resource }: ResourceRowProps): React.ReactNode => {
  return (
    <DataText className="text-xs font-bold">
      {resource.instance_name}{' '}
      {resource.type_name !== resource.instance_name && resource.type_name
        ? `(${resource.type_name})`
        : ''}
    </DataText>
  );
};

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Resource } from '@quent/utils';

interface ResourceRowProps {
  resource: Resource;
}

/** Leaf row displaying a single resource's instance name and type. */
export const ResourceRow = ({ resource }: ResourceRowProps): React.ReactNode => {
  return (
    <div>
      <div>
        <span className="text-xs font-bold">
          {resource.instance_name}{' '}
          {resource.type_name !== resource.instance_name && resource.type_name
            ? `(${resource.type_name})`
            : ''}
        </span>
      </div>
    </div>
  );
};

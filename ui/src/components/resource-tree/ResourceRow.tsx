// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { Resource } from '~quent/types/Resource';

interface ResourceRowProps {
  resource: Resource;
}

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

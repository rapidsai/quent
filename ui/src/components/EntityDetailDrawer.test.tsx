// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { EntityDetailDrawer } from './EntityDetailDrawer';

vi.mock('./entities-table/EntityDetailPanel', () => ({
  EntityDetailPanel: () => <div>Entity detail content</div>,
}));

const fsm = {
  id: 'entity-1',
  type_name: 'Task',
  instance_name: 'Task 1',
  transitions: [],
};
const queryBundle = {} as QueryBundle<EntityRef>;

describe('EntityDetailDrawer', () => {
  it('is non-modal and closes when the background is clicked', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <>
        <button type="button">Background action</button>
        <EntityDetailDrawer
          fsm={fsm}
          resourceLabel={id => id}
          operatorLabel={id => id}
          onClose={onClose}
          queryBundle={queryBundle}
        />
      </>
    );

    expect(screen.getByRole('dialog', { name: 'Entity details' })).not.toHaveAttribute(
      'aria-modal',
      'true'
    );
    expect(document.querySelector('[data-slot="drawer-overlay"]')).not.toBeInTheDocument();

    await waitFor(() => expect(document.body).toHaveStyle({ pointerEvents: 'auto' }));
    await user.click(screen.getByText('Background action'));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it('does not close when a long-entities Gantt entity is clicked', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <>
        <div data-long-entities-gantt>
          <button type="button">Entity bar</button>
        </div>
        <EntityDetailDrawer
          fsm={fsm}
          resourceLabel={id => id}
          operatorLabel={id => id}
          onClose={onClose}
          queryBundle={queryBundle}
        />
      </>
    );

    await waitFor(() => expect(document.body).toHaveStyle({ pointerEvents: 'auto' }));
    await user.click(screen.getByText('Entity bar'));

    expect(onClose).not.toHaveBeenCalled();
  });
});

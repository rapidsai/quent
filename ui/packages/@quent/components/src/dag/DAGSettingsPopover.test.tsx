// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useState } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DAGSettingsPopover } from './DAGSettingsPopover';

const mocks = vi.hoisted(() => ({
  dagControls: vi.fn(),
}));

vi.mock('./DAGControls', () => ({
  DAGControls: (props: {
    operatorStatFields: string[];
    portStatFields: string[];
    isDark: boolean;
  }) => {
    mocks.dagControls(props);
    return <div>DAG controls</div>;
  },
}));

function ControlledSettingsPopover() {
  const [open, setOpen] = useState(false);
  return (
    <DAGSettingsPopover
      operatorStatFields={['duration']}
      portStatFields={['rows']}
      isDark
      open={open}
      onOpenChange={setOpen}
    />
  );
}

describe('DAGSettingsPopover', () => {
  it('opens from the gear trigger and closes on an outside click', async () => {
    const user = userEvent.setup();
    render(
      <>
        <ControlledSettingsPopover />
        <button type="button">Outside target</button>
      </>
    );

    const trigger = screen.getByRole('button', { name: 'Settings' });
    expect(trigger).toHaveAttribute('title', 'Settings');
    expect(screen.queryByText('DAG controls')).not.toBeInTheDocument();

    await user.click(trigger);

    expect(screen.getByText('DAG controls')).toBeInTheDocument();
    expect(mocks.dagControls).toHaveBeenCalledWith({
      operatorStatFields: ['duration'],
      portStatFields: ['rows'],
      isDark: true,
    });

    await user.click(screen.getByRole('button', { name: 'Outside target' }));

    expect(screen.queryByText('DAG controls')).not.toBeInTheDocument();
  });
});

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from '@playwright/test';

test('renders and edits YAML in the production build', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error') {
      errors.push(message.text());
    }
  });

  const response = await page.goto('./?example=simple');
  expect(response?.ok()).toBe(true);

  const editor = page.getByRole('textbox', { name: 'Schema YAML source' });
  await expect(editor).toBeVisible();
  await expect(editor).toContainText('model: AsyncExecutor');

  await editor.click();
  await page.keyboard.press('ControlOrMeta+Home');
  await page.keyboard.insertText('# edited in CI\n');
  await expect(editor).toContainText('# edited in CI');

  await page.getByRole('combobox').first().selectOption('hello');
  await expect(editor).toContainText('model: Hello');
  await expect(editor).not.toContainText('# edited in CI');
  await page.waitForLoadState('networkidle');
  expect(errors).toEqual([]);
});

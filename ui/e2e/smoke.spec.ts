// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from '@playwright/test';

test('loads the query profiler page', async ({ page }) => {
  await page.goto('/');

  await expect(page).toHaveTitle('Quent UI');
  await expect(page.getByRole('heading', { name: 'Query Profiler' })).toBeVisible();
  await expect(page.getByText('Select an engine, coordinator, and query')).toBeVisible();

  await page.getByRole('combobox').first().click();
  await expect(page.getByRole('option', { name: 'test-engine' })).toBeVisible();
  await page.getByRole('option', { name: 'test-engine' }).click();

  await page.getByRole('combobox').nth(1).click();
  await expect(page.getByRole('option', { name: 'test-group' })).toBeVisible();
  await page.getByRole('option', { name: 'test-group' }).click();

  await page.getByRole('combobox').nth(2).click();
  await expect(page.getByRole('option', { name: 'test-query' })).toBeVisible();
});

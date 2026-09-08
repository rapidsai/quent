// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from '@playwright/test';
import { allowedMissingNvtxCatalogErrors } from './utils/allowed-errors';
import { captureErrors, expectNoErrors } from './utils/error-capture';

const ENGINE_ID = '00000000-0000-0000-0000-000000000001';
const QUERY_ID = '00000000-0000-0000-0000-000000000004';
const QUERY_PATH = `/profile/engine/${ENGINE_ID}/query/${QUERY_ID}`;

test('smoke tests the query profiler routes', async ({ page }) => {
  const errors = await captureErrors(page);
  const response = await page.goto('/');

  expect(response?.ok()).toBe(true);

  await expect(page).toHaveTitle('Quent UI');
  await expect(page.getByRole('heading', { name: 'Query Profiler' })).toBeVisible();
  await expect(page.getByText('Select an engine, coordinator, and query')).toBeVisible();
  await page.waitForLoadState('networkidle');
  await expectNoErrors(page, errors, allowedMissingNvtxCatalogErrors);

  await page.getByRole('combobox').first().click();
  await expect(page.getByRole('option', { name: 'test-engine' })).toBeVisible();
  await page.getByRole('option', { name: 'test-engine' }).click();

  await page.getByRole('combobox').nth(1).click();
  await expect(page.getByRole('option', { name: 'test-group' })).toBeVisible();
  await page.getByRole('option', { name: 'test-group' }).click();

  await page.getByRole('combobox').nth(2).click();
  await expect(page.getByRole('option', { name: 'test-query' })).toBeVisible();
  await page.getByRole('option', { name: 'test-query' }).click();

  await expect(page).toHaveURL(new RegExp(`${QUERY_PATH}/timeline$`));
  await expect(page.getByRole('tree').getByText('test-engine', { exact: true })).toBeVisible();
  await page.waitForLoadState('networkidle');
  await expectNoErrors(page, errors, allowedMissingNvtxCatalogErrors);

  const routes = [
    {
      name: 'timeline',
      path: `${QUERY_PATH}/timeline`,
      ready: () => page.getByRole('tree').getByText('test-engine', { exact: true }),
    },
    {
      name: 'operators',
      path: `${QUERY_PATH}/operators`,
      ready: () =>
        page.getByText(/Select a plan on the left to view operators|Worker \/ Plan/).first(),
    },
    {
      name: 'entities',
      path: `${QUERY_PATH}/entities`,
      ready: () => page.getByRole('columnheader', { name: 'Instance' }),
    },
  ];

  for (const route of routes) {
    await test.step(`loads the ${route.name} route directly`, async () => {
      const routeResponse = await page.goto(route.path);

      expect(routeResponse?.ok()).toBe(true);
      await expect(page).toHaveURL(new RegExp(`${route.path}$`));
      await expect(route.ready()).toBeVisible();
      await page.waitForLoadState('networkidle');
      await expectNoErrors(page, errors, allowedMissingNvtxCatalogErrors);
    });
  }
});

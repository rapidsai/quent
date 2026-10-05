// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { expect, test } from '@playwright/test';

const engine = '00000000-0000-0000-0000-000000000001';
const query = '00000000-0000-0000-0000-000000000004';

test('keyboard pipe inspection preserves operator filters and renders nested endpoint data', async ({
  page,
}) => {
  await page.route(`**/api/engines/${engine}/query/${query}`, async route => {
    const response = await route.fetch();
    const bundle = await response.json();
    for (const operator of Object.values(bundle.entities.operators) as Array<
      Record<string, unknown>
    >) {
      operator.custom_attributes = [
        {
          key: 'Decomposition',
          value: {
            Struct: [
              {
                key: 'Second stage',
                value: { Struct: [{ key: 'expression', value: { String: 'price * discount' } }] },
              },
              {
                key: 'First stage',
                value: { Struct: [{ key: 'expression', value: { String: 'quantity > 0' } }] },
              },
            ],
          },
        },
      ];
    }
    for (const port of Object.values(bundle.entities.ports) as Array<Record<string, unknown>>) {
      port.statistics = {
        custom_statistics: [
          {
            key: 'Volume',
            value: {
              Struct: [
                { key: 'bytes', value: { U64: 1024 } },
                { key: 'rows', value: { U64: 10 } },
              ],
            },
          },
        ],
      };
    }
    await route.fulfill({ response, json: bundle });
  });
  await page.goto(`/profile/engine/${engine}/query/${query}/timeline`);
  const node = page.locator('.react-flow__node').first();
  await expect(node).toBeVisible();
  await node.click();
  await expect(page.getByRole('heading', { name: 'Decomposition' }).first()).toBeVisible();
  const headings = page.getByRole('heading').filter({ hasText: /^(Second stage|First stage)$/ });
  expect((await headings.allTextContents()).slice(0, 2)).toEqual(['Second stage', 'First stage']);
  await page.screenshot({ path: '/tmp/quent-operator-decomposition.png', fullPage: true });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('combobox', { name: 'Edge width', exact: true }).click();
  await page.getByRole('option', { name: 'Volume › bytes', exact: true }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect
    .poll(async () =>
      Number(
        await page
          .locator('.react-flow__edge-path')
          .first()
          .evaluate(path => getComputedStyle(path).strokeWidth.replace('px', ''))
      )
    )
    .toBeGreaterThan(2);
  const selectedBefore = await page.locator('.react-flow__node.selected').count();
  const pipe = page.getByRole('button', { name: /^Inspect pipe / }).first();
  await pipe.focus();
  await pipe.press('Enter');
  await expect(page.getByText('Pipe Details', { exact: true })).toBeVisible();
  await expect(
    page.getByRole('region', { name: 'Sending port' }).getByRole('heading', { name: 'Volume' })
  ).toBeVisible();
  await expect(
    page.getByRole('region', { name: 'Receiving port' }).getByText('1.00 KiB')
  ).toBeVisible();
  await expect(pipe).toHaveAttribute('aria-pressed', 'true');
  expect(await page.locator('.react-flow__node.selected').count()).toBe(selectedBefore);
  await page.screenshot({ path: '/tmp/quent-pipe-inspection.png', fullPage: true });
  await node.click();
  await expect(page.getByText('Operator Details', { exact: true })).toBeVisible();
});

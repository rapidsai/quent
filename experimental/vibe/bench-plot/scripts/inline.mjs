// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export function assetReferences(html) {
  const script = html.match(/<script type="module" crossorigin src="\.\/assets\/([^"/]+\.js)"><\/script>/);
  const stylesheet = html.match(/<link rel="stylesheet" crossorigin href="\.\/assets\/([^"/]+\.css)">/);
  if (!script || !stylesheet) throw new Error('Unexpected Vite output for single-page build');
  return { script, stylesheet };
}

export function inlineSinglePage(html, javascript, css, report) {
  const { script, stylesheet } = assetReferences(html);
  const marker = '<!-- quent-bench-single-page -->';
  return html.replace(script[0], () => `<script type="module">${javascript.replace(/<\/script/gi, '<\\/script')}</script>`)
    .replace(stylesheet[0], () => `<style>${css.replace(/<\/style/gi, '<\\/style')}</style>`)
    .replace('</body>', () => `    ${marker}\n    <script id="quent-bench-report" type="application/json">${report.replaceAll('<', '\\u003c')}</script>\n  </body>`);
}

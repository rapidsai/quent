<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import { indentWithTab } from '@codemirror/commands';
  import { yaml } from '@codemirror/lang-yaml';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { keymap } from '@codemirror/view';
  import { basicSetup, EditorView } from 'codemirror';

  interface Props {
    value?: string;
    ariaLabel?: string;
  }

  let {
    value = $bindable(''),
    ariaLabel = 'Schema YAML source',
  }: Props = $props();
  let view = $state<EditorView | null>(null);
  let cursorLine = $state(1);
  let cursorColumn = $state(1);

  function updateCursor(instance: EditorView): void {
    const position = instance.state.selection.main.head;
    const line = instance.state.doc.lineAt(position);
    cursorLine = line.number;
    cursorColumn = position - line.from + 1;
  }

  $effect(() => {
    if (!view || view.state.doc.toString() === value) {
      return;
    }
    view.dispatch({
      changes: {
        from: 0,
        to: view.state.doc.length,
        insert: value,
      },
    });
  });

  function editor(node: HTMLDivElement): { destroy: () => void } {
    const instance = new EditorView({
      doc: value,
      parent: node,
      extensions: [
        basicSetup,
        keymap.of([indentWithTab]),
        yaml(),
        oneDark,
        EditorView.contentAttributes.of({
          'aria-label': ariaLabel,
          autocapitalize: 'off',
          autocomplete: 'off',
          spellcheck: 'false',
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            value = update.state.doc.toString();
          }
          if (update.docChanged || update.selectionSet) {
            updateCursor(update.view);
          }
        }),
      ],
    });
    view = instance;
    updateCursor(instance);
    return {
      destroy: () => {
        view = null;
        instance.destroy();
      },
    };
  }
</script>

<div class="yaml-editor">
  <div class="yaml-editor__surface" use:editor></div>
  <div
    class="flex items-center justify-end gap-3 border-t border-slate-700 bg-slate-900 px-3 py-1 font-mono text-[0.65rem] text-slate-400"
    data-role="editor-position"
  >
    <span>Ln {cursorLine}</span>
    <span>Col {cursorColumn}</span>
  </div>
</div>

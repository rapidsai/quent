/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Static-injection strong-symbol shim (D-15 secondary path).
 *
 * NVTX declares `InitializeInjectionNvtx2_fnptr` as a WEAK symbol
 * (nvtxDetail/nvtxInit.h). A statically-linked injection library "wins" by
 * providing a STRONG definition of that same symbol pointing at its
 * `InitializeInjectionNvtx2` entry. Because our Rust `InitializeInjectionNvtx2`
 * is exported unmangled with C linkage, we only need to publish the strong
 * function-pointer symbol here.
 *
 * Compiled ONLY under the `static-injection` cargo feature via `cc` in
 * build.rs; the default (cdylib / NVTX_INJECTION64_PATH) path never links it,
 * so linkage order is irrelevant for the primary attach path (Pitfall 5).
 */

/* Matches NVTX's NvtxInitializeInjectionNvtxFunc_t:
 *   typedef int (*)(NvtxGetExportTableFunc_t). The export-table func is an
 *   opaque pointer at this boundary, so `void*` is ABI-compatible. */
extern int InitializeInjectionNvtx2(void *get_export_table);

typedef int (*NvtxInitializeInjectionNvtxFunc_t)(void *);

/* Strong (non-weak) definition overriding NVTX's weak symbol. */
NvtxInitializeInjectionNvtxFunc_t InitializeInjectionNvtx2_fnptr =
    (NvtxInitializeInjectionNvtxFunc_t)InitializeInjectionNvtx2;

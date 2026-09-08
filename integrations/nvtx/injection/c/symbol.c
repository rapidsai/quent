/*
 * SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Static-injection strong-symbol shim (secondary attach path).
 *
 * NVTX declares `InitializeInjectionNvtx2_fnptr` as a WEAK symbol
 * (nvtxDetail/nvtxInit.h). A statically-linked injection library "wins" by
 * providing a STRONG definition of that same symbol pointing at its
 * injector entry. Static builds give the Rust entry a Quent-private ABI name;
 * this both avoids colliding with a consumer-owned public trampoline and lets
 * that trampoline forward into the exact same in-process hook state.
 *
 * Compiled ONLY under the `static-injection` cargo feature via `cc` in
 * build.rs; the default (cdylib / NVTX_INJECTION64_PATH) path never links it,
 * so linkage order is irrelevant for the primary attach path.
 */

/* Matches NVTX's NvtxInitializeInjectionNvtxFunc_t:
 *   typedef int (*)(NvtxGetExportTableFunc_t). The export-table func is an
 *   opaque pointer at this boundary, so `void*` is ABI-compatible. */
extern int quent_InitializeInjectionNvtx2(void *get_export_table);

typedef int (*NvtxInitializeInjectionNvtxFunc_t)(void *);

/* Strong (non-weak) definition overriding NVTX's weak symbol. */
NvtxInitializeInjectionNvtxFunc_t InitializeInjectionNvtx2_fnptr =
    (NvtxInitializeInjectionNvtxFunc_t)quent_InitializeInjectionNvtx2;

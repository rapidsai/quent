// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

// Strong definition of InitializeInjectionNvtx2_fnptr that overrides NVTX's
// weak symbol. Delegates to the Rust implementation.

typedef const void* (*NvtxGetExportTableFunc_t)(unsigned int);
typedef int (*NvtxInitializeInjectionNvtxFunc_t)(NvtxGetExportTableFunc_t);

// Implemented in Rust (init.rs).
extern int quent_nvtx_initialize_injection(NvtxGetExportTableFunc_t);

// Strong symbol — linker uses this over NVTX's weak definition.
NvtxInitializeInjectionNvtxFunc_t InitializeInjectionNvtx2_fnptr = quent_nvtx_initialize_injection;

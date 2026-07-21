/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Bindgen umbrella header for the NVTX injection ABI.
 *
 * Including <nvtx3/nvToolsExt.h> transitively pulls in the internal
 * nvtxDetail/nvtxTypes.h (guarded by NVTX_IMPL_GUARD), which defines the
 * injection surface we bind against: the export-table types
 * (NvtxExportTableCallbacks, NvtxExportTableID), the callback-module and
 * per-module CBID enums (NvtxCallbackModule, NvtxCallbackIdCore/Core2),
 * NvtxGetExportTableFunc_t, and the function-table pointer types — alongside
 * the public nvtxEventAttributes_t / nvtxMessageValue_t the CORE/CORE2
 * callbacks receive.
 */
#include <nvtx3/nvToolsExt.h>

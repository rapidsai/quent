#!/bin/sh
# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

if [ "$(uname -s)" = Linux ] && [ -n "${CC:-}" ]; then
    # nvtx-sys uses bindgen, and clang-sys does not find Pixi's libclang by default.
    export LIBCLANG_PATH="${LIBCLANG_PATH:-$CONDA_PREFIX/lib}"
    # Bindgen needs the C headers selected by Pixi's compiler.
    if [ -z "${BINDGEN_EXTRA_CLANG_ARGS:-}" ]; then
        nvtx_gcc_include="$("$CC" -print-file-name=include)"
        nvtx_sysroot="$("$CC" -print-sysroot)"
        nvtx_clang_args=""
        # On aarch64, Pixi's Clang supplies the built-in headers.
        if command -v clang >/dev/null 2>&1; then
            nvtx_clang_args="-isystem \"$(clang -print-resource-dir)/include\" "
        fi
        export BINDGEN_EXTRA_CLANG_ARGS="${nvtx_clang_args}-isystem \"$nvtx_gcc_include\" -isystem \"$nvtx_sysroot/usr/include\""
    fi
fi

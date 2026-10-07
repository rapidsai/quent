# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0


def emit_batch(handle, payloads):
    for _ in payloads:
        pass


durations = measure_threads(config, lambda: None, lambda index: None, emit_batch)
print(json.dumps({"thread_batch_elapsed_ns": durations, "child_pid": os.getpid()}))

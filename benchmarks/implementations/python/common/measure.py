# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

import json
import os
import sys
import threading
from time import perf_counter_ns


def measure_threads(workload, make_handle, prepare, emit_batch):
    """Measure Python call batches with preparation and synchronization outside timing."""
    num_threads = workload["threads"]
    num_batches = workload["num_batches"]
    warmup = workload["num_warmup_batches"]
    total_batches = warmup + num_batches
    batch_size = workload["batch_size"]
    pause_ns = workload["batch_pause_us"] * 1000
    barrier = threading.Barrier(num_threads + 1)
    durations = [[0] * num_batches for _ in range(num_threads)]
    errors = [None] * num_threads

    def worker(index, handle):
        try:
            if workload["preflight_call"]:
                emit_batch(handle, [prepare(0)])
            batches = [
                [prepare(batch * batch_size + i) for i in range(batch_size)]
                for batch in range(total_batches)
            ]
            samples = durations[index]
            barrier.wait()
            for batch in range(total_batches):
                payloads = batches[batch]
                batches[batch] = None
                barrier.wait()
                if batch < warmup:
                    emit_batch(handle, payloads)
                else:
                    start = perf_counter_ns()
                    emit_batch(handle, payloads)
                    elapsed = perf_counter_ns() - start
                    samples[batch - warmup] = elapsed
                # Release batch storage outside timing, before the completion barrier.
                del payloads
                barrier.wait()
                if batch + 1 < total_batches and pause_ns:
                    pause_start = perf_counter_ns()
                    while perf_counter_ns() - pause_start < pause_ns:
                        pass
        except threading.BrokenBarrierError:
            pass
        except BaseException as error:
            errors[index] = error
            barrier.abort()

    workers = []
    try:
        for index in range(num_threads):
            thread = threading.Thread(target=worker, args=(index, make_handle()))
            thread.start()
            workers.append(thread)
        barrier.wait()
        for _ in range(total_batches):
            barrier.wait()
            barrier.wait()
    except threading.BrokenBarrierError:
        pass
    except BaseException:
        barrier.abort()
        raise
    finally:
        for thread in workers:
            thread.join()
    for error in errors:
        if error is not None:
            raise error
    return durations


config = json.loads(sys.argv[1])

# SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from pathlib import Path

import quent_readme as quent


def main() -> None:
    output_dir = Path("./events")
    cluster_id = quent.now_v7()
    context = quent.Context(cluster_id, "ndjson", str(output_dir))

    # The root resource group uses the same ID as the context.
    cluster = context.cluster_observer().cluster(cluster_id, "example_cluster")

    # Spawn a worker.
    worker = context.worker_observer().worker(
        quent.now_v7(),
        "worker_0",
        cluster,
        {
            "version": "42.1.2",
            "custom": {"threads": 256},
        },
    )

    # Construct a queue.
    queue = context.queue_observer().initializing(
        quent.now_v7(),
        "my_queue",
        worker,
    )
    queue.operating(None)

    # Construct a memory pool.
    mem_pool = context.memory_pool_observer().initializing(
        quent.now_v7(),
        "my_memory_pool",
        worker,
    )
    mem_pool.operating(1337)
    mem_pool.resizing()
    mem_pool.operating(2048)

    # Spawn a thread.
    thread = context.thread_observer().initializing(
        quent.now_v7(),
        "my_thread",
        worker,
    )
    thread.operating()

    # Single event entity.
    context.info_observer().info(
        quent.now_v7(),
        "ready to operate",
        __file__,
    )

    # Multi-event entity.
    file_stats = context.file_stats_observer().create(quent.now_v7())
    file_stats.checksum("sha256", "abc123def456")
    file_stats.decompressed("snappy", 0.4)

    # Queue a task. Usage arguments are either a handle, or a tuple whose first
    # element is the handle and remaining elements are capacity values.
    task = context.task_observer().queued(
        quent.now_v7(),
        "my_task_31415",
        1,
        worker,
        (queue, 1),
    )

    task.computing(thread, None)
    task.computing(thread, (mem_pool, 1024))
    task.exit()

    # Close context to flush all pending events.
    context.close()

    output_path = (output_dir / f"{cluster_id}.ndjson").resolve()
    print(f"Events written to: {output_path}")


if __name__ == "__main__":
    main()

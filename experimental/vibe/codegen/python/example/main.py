# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from collections import UserDict
from pathlib import Path

import quent_demo as quent


def main() -> None:
    context = quent.Context(quent.ExporterOptions.ndjson(Path("./events")))
    with context:
        cluster_observer = context.cluster_observer()
        scoped_cluster_telemetry = cluster_observer
        cluster = scoped_cluster_telemetry.create(context.id)
        cluster.declaration(instance_name="example_cluster")
        assert cluster.declaration_emitted()
        try:
            cluster.declaration(instance_name="duplicate_cluster")
        except RuntimeError:
            pass
        else:
            raise AssertionError("once-cardinality event was accepted twice")

        worker = context.worker_observer().create()
        worker.declaration(
            instance_name="worker_0",
            cluster=cluster,
            details=UserDict(
                {
                    "version": "42.1.2",
                    "custom": UserDict({"threads": 256}),
                }
            ),
        )

        queue = context.queue_observer().create()
        queue.declaration(instance_name="my_queue", worker=worker)

        memory = context.memory_pool_observer().create()
        memory.declaration(
            instance_name="my_memory_pool",
            worker=worker,
            limits={"bytes": 1337},
        )
        memory.resized(limits={"bytes": 2048})

        thread = context.thread_observer().create()
        thread.idle(worker=worker)
        thread.active()

        info = context.info_observer().create()
        info.recorded(
            message="ready to operate",
            source=__file__,
            worker=worker,
        )

        file_stats = context.file_stats_observer().create()
        file_stats.scheduled()
        file_stats.checksum(
            details={"algorithm": "sha256", "value": "abc123def456"},
            worker=worker,
        )
        file_stats.decompressed(
            details={"algorithm": "snappy", "ratio": 0.4},
        )

        task = context.task_observer().create()
        task.queued(
            instance_name="my_task_31415",
            index=1,
            worker=worker,
            use_queue=UserDict(
                {
                    "target": queue,
                    "data": UserDict({"entries": 1}),
                }
            ),
        )
        task.computing(
            use_thread={"target": thread, "data": {}},
            use_memory=None,
        )
        task.computing(
            use_thread={"target": thread, "data": {}},
            use_memory={"target": memory, "data": {"bytes": 1024}},
        )
        task.exit()
        thread.idle(worker=worker)
        thread.exit()

        try:
            worker.declaration("worker_1", cluster, {"version": "1", "custom": {}})
        except TypeError:
            pass
        else:
            raise AssertionError("event fields were accepted positionally")

    detached_cluster = cluster_observer.create()
    detached_cluster.declaration(instance_name="detached_cluster")


if __name__ == "__main__":
    main()

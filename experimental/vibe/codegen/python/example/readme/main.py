# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from pathlib import Path

import quent_demo as quent


def main() -> None:
    context = quent.Context(quent.ExporterOptions.ndjson(Path("./events")))
    with context:
        cluster_observer = context.cluster_observer()
        scoped_cluster_telemetry = cluster_observer
        cluster = scoped_cluster_telemetry.handle(context.id)
        cluster.declaration(instance_name="example_cluster")

        worker = context.worker_observer().handle()
        custom: quent.DynamicAttributes = {
            "null": None,
            "bool": True,
            "u8": quent.DynamicValue.u8(8),
            "u16": quent.DynamicValue.u16(16),
            "u32": quent.DynamicValue.u32(32),
            "u64": quent.DynamicValue.u64(64),
            "i8": quent.DynamicValue.i8(-8),
            "i16": quent.DynamicValue.i16(-16),
            "i32": quent.DynamicValue.i32(-32),
            "i64": quent.DynamicValue.i64(-64),
            "f32": quent.DynamicValue.f32(32.5),
            "f64": quent.DynamicValue.f64(64.5),
            "string": quent.DynamicValue.string("value"),
            "structure": quent.DynamicValue.structure({"first": "alpha", "second": 2}),
            "u8_list": quent.DynamicValue.u8_list([1, 2]),
            "u16_list": quent.DynamicValue.u16_list([1, 2]),
            "u32_list": quent.DynamicValue.u32_list([1, 2]),
            "u64_list": quent.DynamicValue.u64_list([1, 2]),
            "i8_list": quent.DynamicValue.i8_list([-1, 2]),
            "i16_list": quent.DynamicValue.i16_list([-1, 2]),
            "i32_list": quent.DynamicValue.i32_list([-1, 2]),
            "i64_list": quent.DynamicValue.i64_list([-1, 2]),
            "f32_list": quent.DynamicValue.f32_list([1.5, 2.5]),
            "f64_list": quent.DynamicValue.f64_list([1.5, 2.5]),
            "string_list": quent.DynamicValue.string_list(["first", "second"]),
            "struct_list": quent.DynamicValue.struct_list(
                [{"name": "first"}, {"name": "second"}]
            ),
            "nested_list": quent.DynamicValue.list(
                [
                    quent.DynamicValue.u8_list([1, 2]),
                    quent.DynamicValue.list(
                        [quent.DynamicValue.string_list(["nested"])]
                    ),
                ]
            ),
        }
        worker.declaration(
            instance_name="worker_0",
            cluster=cluster,
            details={
                "version": "42.1.2",
                "custom": custom,
            },
        )

        queue = context.queue_observer().handle()
        queue.declaration(instance_name="my_queue", worker=worker)

        memory = context.memory_pool_observer().handle()
        memory.declaration(
            instance_name="my_memory_pool",
            worker=worker,
            limits={"bytes": 1337},
        )
        memory.resized(limits={"bytes": 2048})

        thread = context.thread_observer().handle()
        idle_thread = thread.idle(worker=worker)
        active_thread = idle_thread.active()

        info = context.info_observer().handle()
        info.recorded(
            message="ready to operate",
            source=__file__,
            worker=worker,
        )

        file_stats = context.file_stats_observer().handle()
        file_stats.scheduled()
        file_stats.checksum(
            details={"algorithm": "sha256", "value": "abc123def456"},
            worker=worker,
        )
        file_stats.decompressed(
            details={"algorithm": "snappy", "ratio": 0.4},
        )

        task = context.task_observer().handle()
        queued_task = task.queued(
            instance_name="my_task_31415",
            index=1,
            worker=worker,
            use_queue={
                "target": queue,
                "data": {"entries": 1},
            },
        )
        computing_task = queued_task.computing(
            use_thread={"target": active_thread, "data": {}},
            use_memory=None,
        )
        computing_task = computing_task.computing(
            use_thread={"target": active_thread, "data": {}},
            use_memory={"target": memory, "data": {"bytes": 1024}},
        )
        exited_task = computing_task.exit()
        idle_thread = active_thread.idle(worker=worker)
        exited_thread = idle_thread.exit()

    detached_cluster = cluster_observer.handle()
    detached_cluster.declaration(instance_name="detached_cluster")


if __name__ == "__main__":
    main()

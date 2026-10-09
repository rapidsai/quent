# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from collections import UserDict
from pathlib import Path
import uuid

import pytest
import quent_codegen_test as quent


def test_generated_api_accepts_general_mappings(tmp_path: Path) -> None:
    context = quent.Context(quent.ExporterOptions.ndjson(str(tmp_path)))
    assert isinstance(context.id, uuid.UUID)
    assert not context.closed
    assert not hasattr(quent, "Uuid")
    assert not hasattr(quent.ExporterOptions, "none")

    cluster_observer = context.cluster_observer()
    cluster_id = uuid.uuid4()
    cluster = cluster_observer.handle(cluster_id)
    assert cluster.id == cluster_id
    cluster.declaration(instance_name="cluster")
    assert cluster.declaration_emitted()
    with pytest.raises(quent.EventAlreadyEmittedError):
        cluster.declaration(instance_name="duplicate")

    worker = context.worker_observer().handle()
    assert isinstance(worker.id, uuid.UUID)
    worker.declaration(
        instance_name="worker",
        cluster=cluster,
        details=UserDict(
            {
                "version": "1.0",
                "custom": UserDict(
                    {
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
                        "structure": quent.DynamicValue.structure(
                            {"first": "alpha", "second": 2}
                        ),
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
                        "string_list": quent.DynamicValue.string_list(
                            ["first", "second"]
                        ),
                        "struct_list": quent.DynamicValue.struct_list(
                            [{"name": "first"}, {"name": "second"}]
                        ),
                    }
                ),
            }
        ),
    )

    queue = context.queue_observer().handle()
    queue.declaration(instance_name="queue", worker=worker)
    thread = context.thread_observer().handle()
    thread_id = thread.id
    with pytest.raises(AttributeError):
        thread.active()
    idle_thread = thread.idle(worker=worker)
    assert idle_thread.id == thread_id
    active_thread = idle_thread.active()
    with pytest.raises(quent.HandleConsumedError):
        idle_thread.active()

    task = context.task_observer().handle()
    queued_task = task.queued(
        instance_name="task",
        index=1,
        worker=worker,
        use_queue=UserDict(
            {
                "target": queue,
                "data": UserDict({"entries": 1}),
            }
        ),
    )
    computing_task = queued_task.computing(
        use_thread={"target": active_thread, "data": {}},
        use_memory=None,
    )
    computing_task.exit()
    idle_thread = active_thread.idle(worker=worker)
    idle_thread.exit()

    dynamic_thread = context.thread_observer().handle().into_dynamic()
    with pytest.raises(quent.InvalidFsmTransitionError):
        dynamic_thread.active()
    dynamic_thread.idle(worker=worker)
    dynamic_thread.active()
    with pytest.raises(quent.InvalidFsmTransitionError):
        dynamic_thread.active()
    with pytest.raises(quent.InvalidFsmStateError):
        dynamic_thread.try_into_idle()
    active_thread = dynamic_thread.try_into_active()
    with pytest.raises(quent.HandleConsumedError):
        dynamic_thread.try_into_active()
    active_thread.idle(worker=worker)

    context.close()
    assert context.closed
    with pytest.raises(quent.ContextClosedError):
        context.worker_observer()

    detached_cluster = cluster_observer.handle()
    detached_cluster.declaration(instance_name="detached")


def test_noop_skips_fsm_checks() -> None:
    context = quent.Context()
    cluster = context.cluster_observer().handle()
    cluster.declaration(instance_name="cluster")
    with pytest.raises(quent.EventAlreadyEmittedError):
        cluster.declaration(instance_name="duplicate")

    thread = context.thread_observer().handle().into_dynamic()
    thread_id = thread.id

    # Disabled capture permits missing initial events and repeated states.
    thread.active()
    thread.active()
    assert thread.id == thread_id
    with pytest.raises(quent.InvalidFsmStateError):
        thread.try_into_idle()
    active = thread.try_into_active()
    assert active.id == thread_id
    with pytest.raises(quent.HandleConsumedError):
        thread.try_into_active()
    context.close()


def test_dynamic_attributes_preserve_insertion_order(tmp_path: Path) -> None:
    context = quent.Context(quent.ExporterOptions.ndjson(str(tmp_path)))
    worker = context.worker_observer().handle()
    worker.declaration(
        instance_name="ordered",
        cluster=context.id,
        details={
            "version": "1.0",
            "custom": UserDict(
                {
                    "first": 1,
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
                    "structure": UserDict(
                        {"nested_first": 2, "nested_second": 3}
                    ),
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
                    "string_list": quent.DynamicValue.string_list(
                        ["first", "second"]
                    ),
                    "struct_list": quent.DynamicValue.struct_list(
                        [UserDict({"name": "first"}), UserDict({"name": "second"})]
                    ),
                    "nested_list": quent.DynamicValue.list(
                        [
                            quent.DynamicValue.u8_list([1, 2]),
                            quent.DynamicValue.list(
                                [quent.DynamicValue.string_list(["nested"])]
                            ),
                        ]
                    ),
                    "last": 4,
                }
            ),
        },
    )
    context.close()
    del worker

    serialized = "".join(
        path.read_text() for path in tmp_path.rglob("*") if path.is_file()
    )
    keys = [
        "first",
        "u8",
        "u16",
        "u32",
        "u64",
        "i8",
        "i16",
        "i32",
        "i64",
        "f32",
        "f64",
        "string",
        "structure",
        "nested_first",
        "nested_second",
        "u8_list",
        "u16_list",
        "u32_list",
        "u64_list",
        "i8_list",
        "i16_list",
        "i32_list",
        "i64_list",
        "f32_list",
        "f64_list",
        "string_list",
        "struct_list",
        "nested_list",
        "last",
    ]
    positions = [serialized.index(f'"key":"{key}"') for key in keys]
    assert positions == sorted(positions)

    assert (
        '"Struct":[[{"key":"name","value":{"String":"first"}}],'
        '[{"key":"name","value":{"String":"second"}}]]'
        in serialized
    )
    for value in [
        '"U8":8',
        '"U16":16',
        '"U32":32',
        '"U64":64',
        '"I8":-8',
        '"I16":-16',
        '"I32":-32',
        '"I64":-64',
        '"F32":32.5',
        '"F64":64.5',
        '"String":"value"',
        '"U8":[1,2]',
        '"U16":[1,2]',
        '"U32":[1,2]',
        '"U64":[1,2]',
        '"I8":[-1,2]',
        '"I16":[-1,2]',
        '"I32":[-1,2]',
        '"I64":[-1,2]',
        '"F32":[1.5,2.5]',
        '"F64":[1.5,2.5]',
        '"String":["first","second"]',
        '"List":[{"U8":[1,2]},{"List":[{"String":["nested"]}]}]',
    ]:
        assert value in serialized


def test_dynamic_fsm_preserves_transition_sequence(tmp_path: Path) -> None:
    context = quent.Context(quent.ExporterOptions.ndjson(str(tmp_path)))
    worker_id = uuid.uuid4()
    dynamic_thread = (
        context.thread_observer().handle().idle(worker=worker_id).into_dynamic()
    )
    assert isinstance(dynamic_thread.id, uuid.UUID)
    dynamic_thread.active()
    dynamic_thread.idle(worker=worker_id)
    context.close()
    del dynamic_thread

    serialized = "".join(
        path.read_text() for path in tmp_path.rglob("*") if path.is_file()
    )
    for sequence in range(3):
        assert f'"seq":{sequence}' in serialized


def test_dynamic_fsm_handle_is_accepted_as_entity_reference() -> None:
    context = quent.Context()
    worker_id = uuid.uuid4()
    dynamic_thread = (
        context.thread_observer().handle().idle(worker=worker_id).into_dynamic()
    )
    queued_task = context.task_observer().handle().queued(
        instance_name="task",
        index=1,
        worker=worker_id,
        use_queue=None,
    )

    queued_task.computing(
        use_thread={"target": dynamic_thread, "data": {}},
        use_memory=None,
    )

# SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

import pytest

import quent_qe as quent


def test_uuid_ordering() -> None:
    uuid_a = quent.now_v7()
    uuid_b = quent.now_v7()
    assert uuid_a == uuid_a
    assert uuid_a != uuid_b
    assert uuid_a != object()
    with pytest.raises(TypeError):
        uuid_a < uuid_b  # ty:ignore[unsupported-operator]
    with pytest.raises(TypeError):
        uuid_a < object()  # ty:ignore[unsupported-operator]


def test_engine_definition(tmp_path_factory: pytest.TempPathFactory) -> None:
    path = tmp_path_factory.mktemp("events")

    engine_id = quent.now_v7()
    context = quent.Context(engine_id, "ndjson", str(path))

    engine_attrs = {
        "deployment": "test",
        "slots": 8,
        "max_memory_mb": 4096,
        "scale": 1.0,
        "debug": True,
        "default": None,
    }

    engine = context.engine_observer().create(engine_id)
    assert context.id == engine_id
    assert engine.uuid == engine_id
    engine.init(
        {
            "name": "TestEngine",
            "version": "1.0.0",
            "custom_attributes": engine_attrs,
        },
        "engine-0",
    )

    worker = context.worker_observer().create(quent.now_v7())
    worker.init(engine_id, "worker-0")
    worker_id = worker.uuid

    query_group_id = quent.now_v7()
    context.query_group_observer().declaration(
        query_group_id,
        "qg-0",
        engine_id,
    )

    query = context.query_observer().init(
        quent.now_v7(),
        "select-1",
        query_group_id,
    )
    query.planning()
    query.executing()
    query.exit()

    plan_id = quent.now_v7()
    port_src = quent.now_v7()
    port_tgt = quent.now_v7()
    context.plan_observer().declaration(
        plan_id,
        {
            "query_id": quent.nil_uuid(),
            "plan_id": None,
        },
        "physical-plan-0",
        [
            {
                "source": port_src,
                "target": port_tgt,
            }
        ],
        worker_id,
    )

    operator = context.operator_observer().create(quent.now_v7())
    operator_id = operator.uuid
    operator.declaration(
        plan_id,
        [],
        "hash-join-0",
        "HashJoin",
        {
            "algo": "hash_join",
            "selectivity": 0.75,
        },
    )
    operator.statistics({"rows_processed": 10000, "elapsed_ms": 42.5})

    port = context.port_observer().create(quent.now_v7())
    port.declaration(operator_id, "output-0")
    port.statistics({"bytes_transferred": 1048576})

    worker.exit()
    engine.exit()
    context.close()

    output_path = (path / f"{engine_id}.ndjson").resolve()
    assert output_path.exists(), output_path
    assert output_path.stat().st_size > 0, output_path

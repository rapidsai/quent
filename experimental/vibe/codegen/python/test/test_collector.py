# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from pathlib import Path
import socket
import threading
import time
import uuid

import pytest
import quent_codegen_test as quent


def test_collector_server_streams_multiple_sources(tmp_path: Path) -> None:
    def emit_source(address: str, name: str) -> uuid.UUID:
        with quent.Context(quent.ExporterOptions.collector(address)) as context:
            cluster = context.cluster_observer().handle()
            cluster.declaration(instance_name=name)
            return context.id

    with quent.start_collector(quent.ExporterOptions.ndjson(tmp_path)) as collector:
        assert collector.address.startswith("http://127.0.0.1:")
        first = emit_source(collector.address, "first")
        second = emit_source(collector.address, "second")
        assert first != second
        assert not collector.closed

    assert collector.closed
    collector.close()
    for source_id, name in ((first, "first"), (second, "second")):
        output = "\n".join(
            path.read_text() for path in (tmp_path / str(source_id)).rglob("*.ndjson")
        )
        assert f'"instance_name":"{name}"' in output


def test_collector_server_rejects_invalid_startup(tmp_path: Path) -> None:
    output = quent.ExporterOptions.ndjson(tmp_path)
    with pytest.raises(ValueError, match="filesystem exporter"):
        quent.start_collector(quent.ExporterOptions.collector("http://127.0.0.1:1"))
    with pytest.raises(ValueError, match="bind address"):
        quent.start_collector(output, bind_address="invalid")
    with pytest.raises(ValueError, match="advertised_host"):
        quent.start_collector(output, bind_address="0.0.0.0:0")
    with socket.socket() as occupied:
        occupied.bind(("127.0.0.1", 0))
        occupied.listen()
        with pytest.raises(OSError):
            quent.start_collector(
                output, bind_address=f"127.0.0.1:{occupied.getsockname()[1]}"
            )

    with quent.start_collector(
        output, bind_address="0.0.0.0:0", advertised_host="collector.example"
    ) as collector:
        assert collector.address.startswith("http://collector.example:")


@pytest.mark.parametrize("timeout", [None, 5.0])
def test_collector_close_waits_for_active_source(
    tmp_path: Path, timeout: float | None
) -> None:
    collector = quent.start_collector(quent.ExporterOptions.ndjson(tmp_path))
    context = quent.Context(quent.ExporterOptions.collector(collector.address))
    cluster = context.cluster_observer().handle()
    cluster.declaration(instance_name="active")
    source_id = context.id

    started = threading.Event()
    stopped = threading.Event()

    def stop_collector() -> None:
        started.set()
        collector.close(timeout=timeout)
        stopped.set()

    thread = threading.Thread(target=stop_collector, daemon=True)
    thread.start()
    assert started.wait(1)
    assert not stopped.wait(0.05)
    context.close()
    del cluster, context
    assert stopped.wait(5)
    thread.join()
    assert any((tmp_path / str(source_id)).rglob("*.ndjson"))


def test_collector_close_with_timeout_stops_active_source(tmp_path: Path) -> None:
    collector = quent.start_collector(quent.ExporterOptions.ndjson(tmp_path))
    context = quent.Context(quent.ExporterOptions.collector(collector.address))
    cluster = context.cluster_observer().handle()
    cluster.declaration(instance_name="active")

    started = time.monotonic()
    with pytest.raises(TimeoutError, match="collector shutdown timed out"):
        collector.close(timeout=0.05)
    assert time.monotonic() - started < 2
    assert collector.closed
    collector.close(timeout=0)
    collector.close()
    context.close()
    del cluster, context


def test_collector_close_with_timeout_validates_timeout(tmp_path: Path) -> None:
    collector = quent.start_collector(quent.ExporterOptions.ndjson(tmp_path))
    for timeout in (-1, float("nan"), float("inf"), 1e30):
        with pytest.raises(ValueError, match="timeout must be"):
            collector.close(timeout=timeout)
        assert not collector.closed
    collector.close(timeout=5)
    assert collector.closed

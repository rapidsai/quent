# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from pathlib import Path
import struct
import subprocess
import sys
import textwrap

import pytest


@pytest.mark.skipif(
    sys.platform != "linux" or struct.calcsize("P") != 8,
    reason="generated NVTX live capture requires 64-bit Linux",
)
def test_generated_python_captures_nvtx_through_the_common_exporter(
    tmp_path: Path,
) -> None:
    program = textwrap.dedent(
        r"""
        import ctypes
        import importlib
        import json
        import os
        from pathlib import Path
        import sys

        import quent_codegen_nvtx_test as quent

        output = Path(sys.argv[1])
        context = quent.Context(quent.ExporterOptions.ndjson(str(output)))
        context_id = context.id
        process = context.process_observer().handle()
        process_id = process.uuid

        with_source_error = False
        try:
            process.started(process={"native_id": os.getpid() + 1})
        except quent.SourceActivationError:
            with_source_error = True
        assert with_source_error
        assert not process.started_emitted()

        process.started(process={"native_id": os.getpid()})
        native = importlib.import_module(
            "quent_codegen_nvtx_test.quent_codegen_nvtx_test"
        )
        library = ctypes.CDLL(native.__file__)
        emit = library.quent_codegen_nvtx_test_emit
        emit.argtypes = []
        emit.restype = None
        emit()

        del process
        context.close()
        del context

        context_dir = output / str(context_id)

        def read_stream(name):
            return [
                json.loads(line)
                for path in sorted((context_dir / name).iterdir())
                if path.is_file()
                for line in path.read_text().splitlines()
                if line
            ]

        process_events = read_stream("Process")
        assert len(process_events) == 1
        assert process_events[0]["id"] == str(process_id)
        assert process_events[0]["data"]["Started"]["process"]["native_id"] == os.getpid()

        nvtx_events = read_stream("NvtxEvent")
        assert next(iter(nvtx_events[0]["data"])) == "Initialized"
        assert (
            nvtx_events[0]["data"]["Initialized"]["process"]["target"]
            == str(process_id)
        )
        serialized = "\n".join(json.dumps(event["data"]) for event in nvtx_events)
        assert "python-generated-mark" in serialized
        assert "python-generated-range" in serialized
        assert '"Mark"' in serialized
        assert '"RangePush"' in serialized
        assert '"RangePop"' in serialized
        """
    )
    result = subprocess.run(
        [sys.executable, "-c", program, str(tmp_path)],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr

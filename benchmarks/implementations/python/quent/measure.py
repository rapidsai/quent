# SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
# SPDX-License-Identifier: Apache-2.0

from importlib.machinery import ExtensionFileLoader
from importlib.util import module_from_spec, spec_from_file_location


def emit_empty(handle, payloads):
    for _ in payloads:
        handle.instr_call()


def emit_value(handle, payloads):
    for value in payloads:
        handle.instr_call(value=value)


def emit_all(handle, payloads):
    for small, large, short, long in payloads:
        handle.instr_call(small=small, large=large, short=short, long=long)


def short_value(index):
    return "s" * (8 + index % 9)


def long_value(index):
    return "l" * (128 + index % 129)


def run_case():
    options = config["options"]
    name = "quent_bench_python"
    loader = ExtensionFileLoader(name, options["library"])
    spec = spec_from_file_location(name, options["library"], loader=loader)
    module = module_from_spec(spec)
    sys.modules[name] = module
    loader.exec_module(module)
    shape = options["event_shape"]
    api = getattr(module, shape.replace("-", "_"))
    prepare, emit = {
        "empty": (lambda index: None, emit_empty),
        "u8": (lambda index: index % 256, emit_value),
        "u64": (lambda index: index, emit_value),
        "short-string": (short_value, emit_value),
        "long-string": (long_value, emit_value),
        "all": (
            lambda index: (index % 256, index, short_value(index), long_value(index)),
            emit_all,
        ),
    }[shape]
    exporter = options["exporter"]
    context = (
        api.Context()
        if exporter == "noop"
        else api.Context(getattr(api.ExporterOptions, exporter)(options["output"]))
    )
    with context:
        context_id = str(context.id)
        observer = context.entity_observer()
        try:
            durations = measure_threads(config, observer.handle, prepare, emit)
        finally:
            del observer
    return {
        "thread_batch_elapsed_ns": durations,
        "child_pid": os.getpid(),
        "context_id": context_id,
    }


print(json.dumps(run_case()))

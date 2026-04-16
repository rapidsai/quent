// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_model::{Ref, attributes::Attribute, usage, uuid::Uuid};
use quent_readme_example::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = std::path::PathBuf::from("./events");
    let exporter = quent_model::exporter::ExporterOptions::Ndjson(
        quent_model::exporter::NdjsonExporterOptions {
            output_dir: output_dir.clone(),
        },
    );
    let id = Uuid::now_v7();
    let context = AppContext::try_new(id, Some(exporter))?;

    // The root resource group uses the same ID as the context.
    let cluster = context.cluster_observer().cluster(id, "example_cluster");

    // Spawn a worker.
    let worker = context.worker_observer().worker(
        Uuid::now_v7(),
        "worker_0",
        Ref::new(cluster),
        Details {
            version: "42.1.2".to_string(),
            custom: vec![Attribute::u64("threads", 256)].into(),
        },
    );

    // Construct a queue.
    let mut queue = context
        .queue_observer()
        .initializing(Uuid::now_v7(), "my_queue", worker);
    queue.operating(None);

    // Construct a memory pool.
    let mut mem_pool =
        context
            .memory_pool_observer()
            .initializing(Uuid::now_v7(), "my_memory_pool", worker);
    mem_pool.operating(1337);
    mem_pool.resizing();
    mem_pool.operating(2048);

    // Spawn a thread.
    let mut thread = context
        .thread_observer()
        .initializing(Uuid::now_v7(), "my_thread", worker);
    thread.operating();

    // Single event entity
    context.info_observer().info(
        Uuid::now_v7(),
        "ready to operate".to_string(),
        Some(std::file!().to_string()),
    );

    // Multi-event entities can emit in any order from an entity handle.
    let file_stats_handle = context.file_stats_observer().create(Uuid::now_v7());
    file_stats_handle.checksum(Checksum {
        algorithm: "sha256".to_string(),
        value: "abc123def456".to_string(),
    });
    file_stats_handle.decompressed(Decompressed {
        algorithm: "snappy".to_string(),
        ratio: 0.4,
    });

    // Queue a task. The entry transition returns an FSM handle.
    let mut task = context.task_observer().queued(
        Uuid::now_v7(),
        "my_task_31415",
        1,
        Ref::new(worker),
        Some(usage((&queue, 1))),
    );

    task.computing(Some(usage(&thread)), None);
    task.computing(Some(usage(&thread)), Some(usage((&mem_pool, 1024))));
    task.exit();

    // Drop context to flush all pending events.
    drop(context);

    let output_path = output_dir.join(format!("{id}.ndjson"));
    println!(
        "Events written to: {}",
        output_path.canonicalize()?.display()
    );

    Ok(())
}

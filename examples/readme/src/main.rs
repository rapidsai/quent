// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation::{ExporterOptions, FileSystemExporterOptions, FileSystemFormat};
use quent_readme_example::{
    App, Checksum, Cluster, Context, Decompressed, Details, DynamicAttributes, FileStats, Info,
    MemoryPool, MemoryPoolBounds, MemoryPoolUsage, Queue, QueueUsage, Task, Thread, ThreadUsage,
    Worker,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from("./events");
    let context = Context::<App>::try_new(ExporterOptions::FileSystem(
        FileSystemExporterOptions::new(FileSystemFormat::Ndjson, root.clone()),
    ))?;

    // The context generates its own id and writes events under `root/<id>/`.
    // Reuse it as the root entity id.
    let id = context.id();
    let mut cluster = context.observer::<Cluster>().handle_with_id(id);
    cluster.declaration("example_cluster".to_owned())?;

    // Spawn a worker.
    let mut custom = DynamicAttributes::new();
    custom.add_u64("threads", 256);
    let mut worker = context.observer::<Worker>().handle();
    worker.declaration(
        "worker_0".to_owned(),
        cluster.as_entity_ref(),
        Details {
            version: "42.1.2".to_owned(),
            custom,
        },
    )?;

    // Construct a queue with no known upper bound.
    let mut queue = context.observer::<Queue>().handle();
    queue.declaration("my_queue".to_owned(), worker.as_entity_ref())?;

    // Construct and resize a memory pool.
    let mut memory = context.observer::<MemoryPool>().handle();
    memory.declaration(
        "my_memory_pool".to_owned(),
        worker.as_entity_ref(),
        MemoryPoolBounds { bytes: 1337 },
    )?;
    memory.resized(MemoryPoolBounds { bytes: 2048 })?;

    // Spawn a thread.
    let mut thread = context.observer::<Thread>().handle();
    thread.idle(worker.as_entity_ref())?;
    thread.active()?;

    // Emit a structured log event.
    context.observer::<Info>().handle().recorded(
        "ready to operate".to_owned(),
        Some(std::file!().to_owned()),
        worker.as_entity_ref(),
    )?;

    // Multi-event entities can emit their events independently.
    let mut file_stats = context.observer::<FileStats>().handle();
    file_stats.checksum(
        Checksum {
            algorithm: "sha256".to_owned(),
            value: "abc123def456".to_owned(),
        },
        worker.as_entity_ref(),
    )?;
    file_stats.decompressed(Decompressed {
        algorithm: "snappy".to_owned(),
        ratio: 0.4,
    })?;

    // Queue and execute a task with typed resource usage claims.
    let mut task = context.observer::<Task>().handle();
    task.queued(
        "my_task_31415".to_owned(),
        1,
        worker.as_entity_ref(),
        Some(queue.as_entity_ref_with(QueueUsage { entries: 1 })),
    )?;
    task.computing(Some(thread.as_entity_ref_with(ThreadUsage)), None)?;
    task.computing(
        Some(thread.as_entity_ref_with(ThreadUsage)),
        Some(memory.as_entity_ref_with(MemoryPoolUsage { bytes: 1024 })),
    )?;
    task.exit()?;
    thread.idle(worker.as_entity_ref())?;
    thread.exit()?;

    // Flush all entity streams before reporting the output directory.
    drop((
        context, cluster, worker, queue, memory, thread, task, file_stats,
    ));

    let output_dir = root.join(id.to_string());
    println!(
        "Events written to: {}",
        output_dir.canonicalize()?.display()
    );

    Ok(())
}

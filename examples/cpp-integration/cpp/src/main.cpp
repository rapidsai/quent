// Example: C++ application using quent model-generated telemetry API.
//
// Model: Job -> Tasks running on a ThreadPool of Thread resources.
//
// Pipeline: model (Rust) -> quent-codegen -> CXX bridge -> C++ headers -> this

#include "quent-cpp-example-instrumentation/src/bridge/uuid.rs.h"
#include "quent-cpp-example-instrumentation/src/bridge/context.rs.h"
#include "quent-cpp-example-instrumentation/src/bridge/job.rs.h"
#include "quent-cpp-example-instrumentation/src/bridge/thread_pool.rs.h"
#include "quent-cpp-example-instrumentation/src/bridge/task.rs.h"

#include <string>

int main() {
    // Create telemetry context — events exported to ndjson.
    // This also initialises the global event sender used by all observers.
    auto ctx = telemetry::create_context("ndjson", "data");

    // Declare the job (root resource group).
    auto job_obs = telemetry::job::create_observer();
    auto job_id = uuid::now_v7();
    job_obs->submit(job_id, telemetry::job::Submit{
        .name = "batch-42",
        .num_tasks = 4,
    });

    // Declare the thread pool (resource group under job).
    auto pool_obs = telemetry::thread_pool::create_observer();
    auto pool_id = uuid::now_v7();
    pool_obs->thread_pool_init(pool_id, telemetry::thread_pool::ThreadPoolInit{
        .num_threads = 2,
    });

    // Create thread resources (from stdlib, declared separately).
    auto thread_0 = uuid::now_v7();
    auto thread_1 = uuid::now_v7();
    // ... thread resource init/operating events would go here ...

    // Run tasks on the thread pool.
    for (int i = 0; i < 4; i++) {
        // Create task — enters Queued state.
        auto task = telemetry::task::create(telemetry::task::Queued{
            .job_id = job_id,
            .name = "task-" + std::to_string(i),
        });

        // Transition to Running — uses a thread resource.
        auto thread = (i % 2 == 0) ? thread_0 : thread_1;
        task->running(telemetry::task::Running{
            .thread_resource_id = thread,
        });

        // Task exits (could also auto-exit on handle destruction).
        task->exit();
    }

    // Job complete (unit event — no data parameter).
    job_obs->complete(job_id);

    // Context destruction flushes all pending events.
    return 0;
}

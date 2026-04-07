// Example: C++ application using quent model-generated instrumentation API.
//
// Model: Job -> Tasks running on a ThreadPool of Thread resources.
//
// Pipeline: model (Rust) -> quent-codegen -> CXX bridge -> C++ headers -> this

#include "quent-bridge/uuid.rs.h"
#include "quent-bridge/context.rs.h"
#include "quent-bridge/job.rs.h"
#include "quent-bridge/thread_pool.rs.h"
#include "quent-bridge/task.rs.h"

#include <string>

int main() {
    // Create instrumentation context — events exported to ndjson.
    auto ctx = quent::create_context("ndjson", "data");

    // Declare the job (root resource group).
    auto job_obs = quent::job::create_observer();
    auto job_id = uuid::now_v7();
    job_obs->submit(job_id, quent::job::Submit{
        .name = "batch-42",
        .num_tasks = 4,
    });

    // Declare the thread pool (resource group under job).
    auto pool_obs = quent::thread_pool::create_observer();
    auto pool_id = uuid::now_v7();
    pool_obs->thread_pool_init(pool_id, quent::thread_pool::ThreadPoolInit{
        .num_threads = 2,
    });

    // Create thread resources (from stdlib, declared separately).
    auto thread_0 = uuid::now_v7();
    auto thread_1 = uuid::now_v7();
    // ... thread resource init/operating events would go here ...

    // Run tasks on the thread pool.
    for (int i = 0; i < 4; i++) {
        // Create task — enters Queued state.
        auto task = quent::task::create(quent::task::Queued{
            .job_id = job_id,
            .name = "task-" + std::to_string(i),
        });

        // Transition to Running — uses a thread resource.
        auto thread = (i % 2 == 0) ? thread_0 : thread_1;
        task->running(quent::task::Running{
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

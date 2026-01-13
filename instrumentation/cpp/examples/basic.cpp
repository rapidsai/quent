/// This example demonstrates basic usage of the quent C++ instrumentation library.
///
/// It shows how to:
/// - Initialize a quent context with either a collector or ndjson exporter
/// - Create and observe an engine
/// - Create workers
/// - Create query groups and queries
/// - Transition entities through their lifecycle states

#include <iostream>
#include <thread>
#include <chrono>
#include <string>
#include <filesystem>

#include "quent-cpp/src/lib.rs.h"
#include "quent-cpp/src/uuid.rs.h"
#include "quent-cpp/src/engine.rs.h"
#include "quent-cpp/src/worker.rs.h"
#include "quent-cpp/src/query_group.rs.h"
#include "quent-cpp/src/query.rs.h"

// Helper to simulate work
void simulate_work(int milliseconds) {
    std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
}

// Helper to convert rust::String to std::string for output
std::string to_std_string(const ::rust::String& rs) {
    return std::string(rs.data(), rs.size());
}

int main() {
    // Generate a unique engine ID
    auto engine_id = uuid::UUID::now_v7();

    std::cerr << "Starting basic quent instrumentation example" << std::endl;
    std::cerr << "Engine ID: " << to_std_string(engine_id.to_string()) << std::endl;

    // Create data directory for ndjson output if it doesn't exist
    std::filesystem::create_directories("data");

    // Initialize the quent context
    // Option 1: Use ndjson exporter (writes to stdout as newline-delimited JSON)
    // This is the default for easy testing without requiring a collector
    auto context = quent::quent_context::initialize_with_ndjson_exporter(engine_id);

    // Option 2: Use collector exporter (sends data to a quent collector)
    // Pass empty string for default collector address, or specify a custom one
    // auto context = quent::quent_context::initialize_with_collector_exporter(
    //     engine_id,
    //     "" // Use default collector address
    // );

    // Get the engine observer and initialize the engine
    auto& engine_obs = context->engine_observer();

    quent::engine::engine_implementation_attributes impl_attrs;
    impl_attrs.name = "BasicExample";
    impl_attrs.version = "1.0.0";

    quent::engine::init engine_init;
    engine_init.implementation = impl_attrs;
    engine_init.name = "basic-example-engine";

    engine_obs.init(engine_id, engine_init);
    std::cerr << "Engine initialized" << std::endl;

    // Transition engine to operating state
    engine_obs.operating(engine_id);
    std::cerr << "Engine operating" << std::endl;

    // Create a worker
    auto worker_id = uuid::UUID::now_v7();
    auto& worker_obs = context->worker_observer();

    quent::worker::init worker_init;
    worker_init.engine_id = engine_id;
    worker_init.name = "worker-0";

    worker_obs.init(worker_id, worker_init);
    worker_obs.operating(worker_id);
    std::cerr << "Worker created and operating: " << to_std_string(worker_id.to_string()) << std::endl;

    // Create a query group
    auto query_group_id = uuid::UUID::now_v7();
    auto& query_group_obs = context->query_group_observer();

    quent::query_group::init qg_init;
    qg_init.engine_id = engine_id;
    qg_init.name = "example-query-group";

    query_group_obs.init(query_group_id, qg_init);
    query_group_obs.operating(query_group_id);
    std::cerr << "Query group created: " << to_std_string(query_group_id.to_string()) << std::endl;

    // Create and execute a query
    auto query_id = uuid::UUID::now_v7();
    auto& query_obs = context->query_observer();

    quent::query::init query_init;
    query_init.query_group_id = query_group_id;
    query_init.name = "SELECT * FROM example";

    query_obs.init(query_id, query_init);
    std::cerr << "Query created: " << to_std_string(query_id.to_string()) << std::endl;

    // Simulate query lifecycle
    query_obs.planning(query_id);
    std::cerr << "Query planning..." << std::endl;
    simulate_work(100);

    query_obs.executing(query_id);
    std::cerr << "Query executing..." << std::endl;
    simulate_work(200);

    query_obs.finalizing(query_id);
    std::cerr << "Query finalizing..." << std::endl;
    simulate_work(50);

    query_obs.exit(query_id);
    std::cerr << "Query completed" << std::endl;

    // Clean up query group
    query_group_obs.finalizing(query_group_id);
    query_group_obs.exit(query_group_id);
    std::cerr << "Query group finalized" << std::endl;

    // Clean up worker
    worker_obs.finalizing(worker_id);
    worker_obs.exit(worker_id);
    std::cerr << "Worker finalized" << std::endl;

    // Clean up engine
    engine_obs.finalizing(engine_id);
    engine_obs.exit(engine_id);
    std::cerr << "Engine finalized" << std::endl;

    std::cerr << "Example completed successfully!" << std::endl;
    return 0;
}

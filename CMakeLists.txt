cmake_minimum_required(VERSION 3.20)
project(quent_cpp)

include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5 # Using v0.6 or above does not work because
    # corrosion uses some hash dir to place the generated headers,
    # because of which target_include_directories below does not work.
    # Fix? Upstream a solution to Corrosion so appropriate vars are set
    # which can then be used.
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH instrumentation/cpp/Cargo.toml)
target_include_directories(quent_cpp INTERFACE "${CMAKE_BINARY_DIR}/cargo/build/${Rust_CARGO_TARGET_CACHED}/cxxbridge")

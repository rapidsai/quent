cmake_minimum_required(VERSION 3.24)
project(quent-qe-cpp-example LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Corrosion: build Rust crates from CMake
include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.6.1
)
FetchContent_MakeAvailable(Corrosion)

# Import the Rust bridge crate (CXX bridges)
corrosion_import_crate(
    MANIFEST_PATH ${CMAKE_SOURCE_DIR}/../bridge/Cargo.toml
    CRATES quent-qe-bridge
)

# The C++ application
add_executable(example src/main.cpp)

# Link against the Rust static library
target_link_libraries(example PRIVATE quent_qe_bridge)

# CXX-generated headers are copied to bridge/include/ by build.rs.
# This directory contains rust/cxx.h and the per-module .rs.h headers.
target_include_directories(example PRIVATE
    ${CMAKE_SOURCE_DIR}/../bridge/include
)

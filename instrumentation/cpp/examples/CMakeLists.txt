cmake_minimum_required(VERSION 3.20)
project(quent_examples LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)
FetchContent_Declare(
    quent_cpp
    SOURCE_DIR ../../../../
)
FetchContent_MakeAvailable(quent_cpp)

add_executable(basic basic.cpp)

target_link_libraries(basic PRIVATE
    quent_cpp
)

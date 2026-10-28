cmake_minimum_required(VERSION 3.20)
project(kamafeu_console_tui LANGUAGES CXX)

include(FetchContent)
set(FTXUI_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(FTXUI_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(FTXUI_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(FTXUI_ENABLE_INSTALL OFF CACHE BOOL "" FORCE)
set(FTXUI_QUIET ON CACHE BOOL "" FORCE)

FetchContent_Declare(
  ftxui
  GIT_REPOSITORY https://github.com/ArthurSonzogni/FTXUI.git
  GIT_TAG v7.0.3
  GIT_SHALLOW TRUE
)
FetchContent_MakeAvailable(ftxui)

add_executable(kamafeu-tui main.cpp)
target_compile_features(kamafeu-tui PRIVATE cxx_std_17)
target_link_libraries(kamafeu-tui PRIVATE ftxui::ftxui)
if(WIN32)
  target_link_libraries(kamafeu-tui PRIVATE ws2_32)
endif()
set_target_properties(kamafeu-tui PROPERTIES
  RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
  RUNTIME_OUTPUT_DIRECTORY_RELEASE "${CMAKE_BINARY_DIR}/bin/Release"
)

include(CTest)
if(BUILD_TESTING AND UNIX)
  find_package(Python3 COMPONENTS Interpreter REQUIRED)
  add_test(
    NAME kamafeu_ftxui_transport
    COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/smoke.py"
            "$<TARGET_FILE:kamafeu-tui>"
  )
endif()

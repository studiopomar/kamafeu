add_library(lessampler_tainah SHARED tainah.cpp)
target_include_directories(lessampler_tainah PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(lessampler_tainah PRIVATE AudioModel AudioModelSynthesis WorldModule)
target_compile_definitions(lessampler_tainah PRIVATE LESSAMPLER_TAINAH_BUILD)
set_target_properties(lessampler_tainah PROPERTIES
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN YES
    VERSION 1.0.0
    SOVERSION 1
)
include(GNUInstallDirs)
install(TARGETS lessampler_tainah
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES tainah.h DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/lessampler-tainah)

if (BUILD_TESTING)
    find_package(Threads REQUIRED)
    add_executable(lessampler_tainah_abi_test tainah_abi_test.cpp)
    target_link_libraries(lessampler_tainah_abi_test PRIVATE lessampler_tainah Threads::Threads)
    add_test(NAME lessampler_tainah_abi COMMAND lessampler_tainah_abi_test)
endif ()

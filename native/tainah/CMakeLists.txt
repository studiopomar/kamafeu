add_library(tainah_motor SHARED tainah.cpp)
target_include_directories(tainah_motor PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(tainah_motor PRIVATE AudioModel AudioModelSynthesis WorldModule)
target_compile_definitions(tainah_motor PRIVATE MOTOR_TAINAH_BUILD)
set_target_properties(tainah_motor PROPERTIES
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN YES
    VERSION 1.0.0
    SOVERSION 1
)
include(GNUInstallDirs)
install(TARGETS tainah_motor
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(FILES tainah.h DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/tainah-motor)

if (BUILD_TESTING)
    find_package(Threads REQUIRED)
    add_executable(tainah_abi_test tainah_abi_test.cpp)
    target_link_libraries(tainah_abi_test PRIVATE tainah_motor Threads::Threads)
    add_test(NAME tainah_abi COMMAND tainah_abi_test)
endif ()

# The browsing engine: a QML module Quickshell imports as Isle.Files (D66).

cmake_minimum_required(VERSION 3.28)
project(isle-files VERSION 0.1.0 LANGUAGES CXX)

find_package(Qt6 6.8 REQUIRED COMPONENTS Core Qml)
qt_standard_project_setup(REQUIRES 6.8)

# Where the shell's QML_IMPORT_PATH points; install.sh passes the installed copy's.
set(ISLE_QML_DIR "$ENV{HOME}/.local/share/isle/qml" CACHE PATH "Import path the shell reads")

set(ISLE_FILES_QML_OUT "${CMAKE_BINARY_DIR}/qml/Isle/Files")

# Naming the plugin after the target makes one library rather than a plugin over a backing library,
# so the whole module is the three files that land in the import path.
qt_add_qml_module(isle-files
    URI Isle.Files
    VERSION 1.0
    PLUGIN_TARGET isle-files
    OUTPUT_DIRECTORY "${ISLE_FILES_QML_OUT}"
    SOURCES
        engine.h
        engine.cpp
)

target_compile_definitions(isle-files PRIVATE ISLE_FILES_VERSION="${PROJECT_VERSION}")
target_link_libraries(isle-files PRIVATE Qt6::Core Qt6::Qml)

# The plugin, its qmldir and its type information; the .qrc beside them is the build's own.
install(DIRECTORY "${ISLE_FILES_QML_OUT}/" DESTINATION "${ISLE_QML_DIR}/Isle/Files"
        PATTERN "*.qrc" EXCLUDE)

cmake_minimum_required(VERSION 3.19)

project(phototux_qml_bundle LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 6.10 REQUIRED COMPONENTS Qml Quick)
qt_standard_project_setup(REQUIRES 6.10)

set(QML_ROOT "${CMAKE_CURRENT_LIST_DIR}/../../../qml")
set(QML_FILES
    "${QML_ROOT}/Main.qml"
    "${QML_ROOT}/NewDocumentDialog.qml"
)
set(ICON_ROOT "${CMAKE_CURRENT_LIST_DIR}/../../../assets/icons/phosphor/regular")
set(ICON_NAMES
    arrow-clockwise
    arrow-counter-clockwise
    circle-notch
    corners-in
    eraser
    export
    eye
    eye-slash
    file-plus
    floppy-disk
    folder-open
    hand
    info
    magnifying-glass
    paint-brush
    x
)
set(ICON_FILES)
foreach(ICON_NAME IN LISTS ICON_NAMES)
    set(ICON_FILE "${ICON_ROOT}/${ICON_NAME}.svg")
    list(APPEND ICON_FILES "${ICON_FILE}")
    set_source_files_properties(
        "${ICON_FILE}"
        PROPERTIES QT_RESOURCE_ALIAS "icons/${ICON_NAME}.svg"
    )
endforeach()

set_source_files_properties(
    "${QML_ROOT}/Main.qml"
    PROPERTIES QT_RESOURCE_ALIAS "Main.qml"
)
set_source_files_properties(
    "${QML_ROOT}/NewDocumentDialog.qml"
    PROPERTIES QT_RESOURCE_ALIAS "NewDocumentDialog.qml"
)

qt_add_qml_module(phototux_qml
    STATIC
    PLUGIN_TARGET phototux_qml
    URI PhotoTux.App
    VERSION 1.0
    CLASS_NAME PhotoTuxQmlPlugin
    OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/qml/PhotoTux/App"
    QML_FILES
        ${QML_FILES}
    RESOURCES
        ${ICON_FILES}
    SOURCES
        phototux_qml_anchor.cpp
)

set_target_properties(phototux_qml PROPERTIES
    ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/lib"
)

target_link_libraries(phototux_qml PRIVATE Qt6::Quick)

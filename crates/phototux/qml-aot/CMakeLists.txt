cmake_minimum_required(VERSION 3.19)

project(phototux_qml_bundle LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 6.10 REQUIRED COMPONENTS Qml Quick)
qt_standard_project_setup(REQUIRES 6.10)

set(QML_ROOT "${CMAKE_CURRENT_LIST_DIR}/../../../qml")

# Every .qml under qml/ is embedded. Globbing here rather than listing names
# keeps a new component from silently missing the AOT module; CONFIGURE_DEPENDS
# re-runs CMake when the set of files changes.
file(GLOB QML_FILES CONFIGURE_DEPENDS "${QML_ROOT}/*.qml")
list(LENGTH QML_FILES QML_FILE_COUNT)
if(QML_FILE_COUNT EQUAL 0)
    message(FATAL_ERROR "No QML files found under ${QML_ROOT}")
endif()

# Singletons must be declared as such or `pragma Singleton` types fail to resolve.
set(QML_SINGLETONS Theme.qml)

foreach(QML_FILE IN LISTS QML_FILES)
    get_filename_component(QML_NAME "${QML_FILE}" NAME)
    set_source_files_properties("${QML_FILE}" PROPERTIES QT_RESOURCE_ALIAS "${QML_NAME}")
    if(QML_NAME IN_LIST QML_SINGLETONS)
        set_source_files_properties("${QML_FILE}" PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
    endif()
endforeach()
set(ICON_ROOT "${CMAKE_CURRENT_LIST_DIR}/../../../assets/icons/phosphor/regular")
# Every icon key any descriptor or action names must appear here, or the
# control ships blank. `every_icon_key_is_packaged_into_the_qrc` in
# phototux_engine reads this list as text and fails when one is missing —
# CMake cannot see the Rust descriptors, so the check runs from the other side.
set(ICON_NAMES
    arrow-clockwise
    arrows-clockwise
    arrow-counter-clockwise
    arrow-down
    arrow-elbow-down-right
    arrow-square-out
    arrow-up
    arrows-in-line-vertical
    arrows-left-right
    arrows-out
    arrows-out-cardinal
    arrows-out-line-vertical
    caret-down
    caret-right
    caret-up
    circle-dashed
    circle-half
    circle-half-tilt
    circle-notch
    corners-in
    crop
    dots-three
    eraser
    export
    eye
    eye-slash
    eyedropper
    file-plus
    floppy-disk
    folder
    folder-open
    gear
    gradient
    hand
    image-square
    intersect
    info
    lasso
    magic-wand
    magnifying-glass
    minus
    minus-circle
    minus-square
    monitor
    paint-brush
    paint-bucket
    pen-nib
    plus
    polygon
    question
    rectangle-dashed
    selection
    selection-background
    selection-foreground
    selection-plus
    shapes
    text-t
    trash
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

set(LOGO_UI "${CMAKE_CURRENT_LIST_DIR}/../../../assets/logo-ui.png")
list(APPEND ICON_FILES "${LOGO_UI}")
set_source_files_properties(
    "${LOGO_UI}"
    PROPERTIES QT_RESOURCE_ALIAS "logo-ui.png"
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

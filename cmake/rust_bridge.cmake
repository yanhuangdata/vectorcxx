# Creates a target including rust lib and cxxbridge named ${NAMESPACE}::${NAME}
function(add_library_rust)
    # set(OPTIONS)
    set(ONE_VALUE_KEYWORDS NAMESPACE NAME)
    # set(MULTI_VALUE_KEYWORDS)
    cmake_parse_arguments(_RUST_LIB "${OPTIONS}" "${ONE_VALUE_KEYWORDS}" "${MULTI_VALUE_KEYWORDS}" ${ARGN})


    ### Check inputs
    if("${_RUST_LIB_NAME}" STREQUAL "")
        message(FATAL_ERROR "add_library_rust called without a given name, fix by adding 'NAME <RustlibName>'")
    endif()

    if("${_RUST_LIB_NAMESPACE}" STREQUAL "")
        message(FATAL_ERROR "Must supply a namespace given by keyvalue NAMESPACE <value>")
    endif()

    if(NOT EXISTS "${CMAKE_CURRENT_LIST_DIR}/../Cargo.toml")
        message(FATAL_ERROR "The path ${CMAKE_CURRENT_LIST_DIR} does not contain a Cargo.toml")
    endif()

    ## Simplyfy inputs
    set(_LIB_NAME ${_RUST_LIB_NAME})
    set(_NAMESPACE ${_RUST_LIB_NAMESPACE})

    ## Import Rust target
    corrosion_import_crate(MANIFEST_PATH "${CMAKE_CURRENT_LIST_DIR}/../Cargo.toml")

    ## Set cxxbridge values

    set(CXXBRIDGE_BINARY_FOLDER ${CMAKE_BINARY_DIR}/cargo/build/${Rust_CARGO_TARGET}/cxxbridge)
    set(ORIGIN_COMMON_HEADER ${CXXBRIDGE_BINARY_FOLDER}/rust/cxx.h)
    set(ORIGIN_BINDING_HEADER ${CXXBRIDGE_BINARY_FOLDER}/${_LIB_NAME}/src/lib.rs.h)
    set(ORIGIN_BINDING_SOURCE ${CXXBRIDGE_BINARY_FOLDER}/${_LIB_NAME}/src/lib.rs.cc)

    ## Create cxxbridge target
    add_custom_command(
            DEPENDS ${_LIB_NAME}-static
            OUTPUT
                ${ORIGIN_COMMON_HEADER}
                ${ORIGIN_BINDING_HEADER}
                ${ORIGIN_BINDING_SOURCE}
    )

    set(GENERATED_SRC_DIR ${CMAKE_CURRENT_LIST_DIR}/../generated_src)
    set(CXX_BINDING_INCLUDE_DIR ${GENERATED_SRC_DIR})
    set(COMMON_HEADER ${GENERATED_SRC_DIR}/rust/cxx.h)
    set(BINDING_HEADER ${GENERATED_SRC_DIR}/${_LIB_NAME}/src/lib.rs.h)
    set(BINDING_SOURCE ${GENERATED_SRC_DIR}/${_LIB_NAME}/src/lib.rs.cc)

    add_custom_command(
            COMMAND ${CMAKE_COMMAND} -E copy_if_different ${ORIGIN_COMMON_HEADER} ${COMMON_HEADER}
            COMMAND ${CMAKE_COMMAND} -E copy_if_different ${ORIGIN_BINDING_HEADER} ${BINDING_HEADER}
            COMMAND ${CMAKE_COMMAND} -E copy_if_different ${ORIGIN_BINDING_SOURCE} ${BINDING_SOURCE}
            DEPENDS ${ORIGIN_COMMON_HEADER}
            OUTPUT
                ${COMMON_HEADER}
                ${BINDING_SOURCE}
                ${BINDING_HEADER}
    )

    set(CXXBRIDGE_TARGET ${_LIB_NAME}-bridge)
    add_library(${CXXBRIDGE_TARGET})

    if(NOT DEFINED VCPKG_TARGET_TRIPLET)
        if(APPLE)
            set(VCPKG_TARGET_TRIPLET "x64-osx")
        else()
            set(VCPKG_TARGET_TRIPLET "x64-linux")
        endif()
    endif()

    # if OPENSSL_INCLUDE_DIR env var is defined and is not empty string
    if(DEFINED ENV{OPENSSL_INCLUDE_DIR} AND NOT "$ENV{OPENSSL_INCLUDE_DIR}" STREQUAL "")
        set(VECTOR_OPENSSL_INCLUDE_DIR $ENV{OPENSSL_INCLUDE_DIR})
        message(STATUS "Use OPENSSL_INCLUDE_DIR in environment var")
    else()
        set(MANIFEST_MODE_VCPKG_OPENSSL_INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/vcpkg_installed/${VCPKG_TARGET_TRIPLET}/include)
        if(EXISTS ${MANIFEST_MODE_VCPKG_OPENSSL_INCLUDE_DIR})
            set(VECTOR_OPENSSL_INCLUDE_DIR ${MANIFEST_MODE_VCPKG_OPENSSL_INCLUDE_DIR})
            message(STATUS "Found openssl include dir via vcpkg in manifeste mode")
        else()
            message(STATUS "Could not find openssl include dir via vcpkg in manifeste mode MANIFEST_MODE_VCPKG_OPENSSL_INCLUDE_DIR=${MANIFEST_MODE_VCPKG_OPENSSL_INCLUDE_DIR}")
            set(VECTOR_OPENSSL_INCLUDE_DIR $ENV{VCPKG_ROOT}/installed/${VCPKG_TARGET_TRIPLET}/include/)
            message(STATUS "Found openssl include dir via vcpkg in classic mode")
        endif()
    endif()
    message(STATUS "Using openssl include dir: OPENSSL_INCLUDE_DIR=${VECTOR_OPENSSL_INCLUDE_DIR}")

    set_property(
            TARGET vectorcxx
            APPEND
            PROPERTY CORROSION_ENVIRONMENT_VARIABLES
            "OPENSSL_INCLUDE_DIR=${VECTOR_OPENSSL_INCLUDE_DIR}"
    )

    if(DEFINED ENV{PKG_CONFIG_PATH} AND NOT "$ENV{PKG_CONFIG_PATH}" STREQUAL "")
        set(VECTOR_PKG_CONFIG_PATH $ENV{PKG_CONFIG_PATH})
        message(STATUS "Use PKG_CONFIG_PATH in environment var")
    else()
        set(MANIFEST_MODE_PKG_CONFIG_PATH ${CMAKE_CURRENT_BINARY_DIR}/vcpkg_installed/${VCPKG_TARGET_TRIPLET}/lib/pkgconfig)
        if(EXISTS ${MANIFEST_MODE_PKG_CONFIG_PATH})
            set(VECTOR_PKG_CONFIG_PATH ${MANIFEST_MODE_PKG_CONFIG_PATH})
            message(STATUS "Found PKG_CONFIG_PATH via vcpkg in manifeste mode")
        else()
            message(STATUS "Could not find PKG_CONFIG_PATH via vcpkg in manifeste mode MANIFEST_MODE_PKG_CONFIG_PATH=${MANIFEST_MODE_PKG_CONFIG_PATH}")
            set(VECTOR_OPKG_CONFIG_PATH $ENV{VCPKG_ROOT}/installed/${VCPKG_TARGET_TRIPLET}/lib/pkgconfig/)
            message(STATUS "Found PKG_CONFIG_PATH via vcpkg in classic mode")
        endif()
    endif()
    message(STATUS "Using PKG_CONFIG_PATH: PKG_CONFIG_PATH=${VECTOR_PKG_CONFIG_PATH}")

    set_property(
            TARGET vectorcxx
            APPEND
            PROPERTY CORROSION_ENVIRONMENT_VARIABLES
            "PKG_CONFIG_PATH=${VECTOR_PKG_CONFIG_PATH}"
    )

    target_sources(${CXXBRIDGE_TARGET}
            PRIVATE
                ${COMMON_HEADER}
                ${BINDING_HEADER}
                ${BINDING_SOURCE}
            )

    target_include_directories(${CXXBRIDGE_TARGET}
            PUBLIC
                $<BUILD_INTERFACE:${CXX_BINDING_INCLUDE_DIR}>
                $<INSTALL_INTERFACE:include>
            )

    ## Create total target with alias with given namespace
    set(CXXBRIDGE_TOTAL_TARGET ${_LIB_NAME}-total)

    add_library(${CXXBRIDGE_TOTAL_TARGET} INTERFACE)
    target_link_libraries(${CXXBRIDGE_TOTAL_TARGET}
            INTERFACE
                ${CXXBRIDGE_TARGET}
                ${_LIB_NAME}
            )
    # for end-user to link into project
    message(STATUS "add_library ${_NAMESPACE}::${_LIB_NAME}")
    add_library(${_NAMESPACE}::${_LIB_NAME} ALIAS ${CXXBRIDGE_TOTAL_TARGET})

    install(TARGETS ${_LIB_NAME}
            EXPORT ${EXPORT_TARGET_NAME}
            )
    install(TARGETS ${CXXBRIDGE_TARGET}
            EXPORT ${EXPORT_TARGET_NAME}
            )
    install(TARGETS ${CXXBRIDGE_TOTAL_TARGET}
            EXPORT ${EXPORT_TARGET_NAME}
            )

endfunction(add_library_rust)

# TODO: handle arm64 architecture
if("${CMAKE_SYSTEM_NAME}" STREQUAL "Windows")
    set(Rust_CARGO_TARGET "x86_64-pc-windows-gnu")
elseif("${CMAKE_SYSTEM_NAME}" STREQUAL "Linux")
    set(Rust_CARGO_TARGET "x86_64-unknown-linux-gnu")
elseif("${CMAKE_SYSTEM_NAME}" STREQUAL "Darwin")
    # on macOS "uname -m" returns the architecture (x86_64 or arm64)
    execute_process(
        COMMAND uname -m
        RESULT_VARIABLE exit_code_or_error
        OUTPUT_VARIABLE OSX_NATIVE_ARCHITECTURE
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    if(OSX_NATIVE_ARCHITECTURE STREQUAL "arm64")
        set(Rust_CARGO_TARGET "aarch64-apple-darwin")
    else()
        set(Rust_CARGO_TARGET "x86_64-apple-darwin")
    endif()
else()
    message(FATAL_ERROR "hardcoded ${CMAKE_SYSTEM_NAME} platformchecks not supported outside windows-gnu, linux-gnu and apple-darwin")
endif()

find_package(Corrosion REQUIRED)
add_library_rust(NAME vectorcxx NAMESPACE vectorcxx)
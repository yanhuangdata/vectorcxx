# Creates a target including rust lib and cxxbridge named ${NAMESPACE}::${NAME}
function(find_dependency_path dependency_path_name dependency_location RETURN_DEPENDENCY_PATH)
    if(DEFINED ENV{${dependency_path_name}} AND NOT "$ENV{${dependency_path_name}}" STREQUAL "")
        set(VECTOR_${dependency_path_name} $ENV{${dependency_path_name}})
        message(STATUS "Use ${dependency_path_name} in environment var")
    else()
        set(MANIFEST_MODE_${dependency_path_name} ${CMAKE_CURRENT_BINARY_DIR}/vcpkg_installed/${VCPKG_TARGET_TRIPLET}/${dependency_location})
        if(EXISTS ${MANIFEST_MODE_${dependency_path_name}})
            set(VECTOR_${dependency_path_name} ${MANIFEST_MODE_${dependency_path_name}})
            message(STATUS "Found ${dependency_path_name} via vcpkg in manifest mode")
        else()
            message(STATUS "Could not find ${dependency_path_name} via vcpkg in manifest mode MANIFEST_MODE_${dependency_path_name}=${MANIFEST_MODE_${dependency_path_name}}")
            set(VECTOR_${dependency_path_name} $ENV{VCPKG_ROOT}/installed/${VCPKG_TARGET_TRIPLET}/${dependency_location})
            message(STATUS "Found ${dependency_path_name} via vcpkg in classic mode")
        endif()
    endif()
    message(STATUS "Using ${dependency_path_name}: ${dependency_path_name}=${VECTOR_${dependency_path_name}}")
    set(${RETURN_DEPENDENCY_PATH} ${VECTOR_${dependency_path_name}} PARENT_SCOPE)
endfunction()

function(update_target_properties)
    find_dependency_path(OPENSSL_INCLUDE_DIR include VECTOR_OPENSSL_INCLUDE_DIR)
    find_dependency_path(PKG_CONFIG_PATH lib/pkgconfig VECTOR_PKG_CONFIG_PATH)
    find_dependency_path(PROTOC_PATH tools/protobuf/protoc VECTOR_PROTOC_PATH)

    set_property(
        TARGET vectorcxx
        APPEND
        PROPERTY CORROSION_ENVIRONMENT_VARIABLES
        "OPENSSL_NO_VENDOR=true"
    )

    set_property(
        TARGET vectorcxx
        APPEND
        PROPERTY CORROSION_ENVIRONMENT_VARIABLES
        "OPENSSL_INCLUDE_DIR=${VECTOR_OPENSSL_INCLUDE_DIR}"
    )

    set_property(
        TARGET vectorcxx
        APPEND
        PROPERTY CORROSION_ENVIRONMENT_VARIABLES
        "PKG_CONFIG_PATH=${VECTOR_PKG_CONFIG_PATH}"
    )

    set_property(
        TARGET vectorcxx
        APPEND
        PROPERTY CORROSION_ENVIRONMENT_VARIABLES
        "PROTOC=${VECTOR_PROTOC_PATH}"
    )
endfunction()

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

    set(CRATE_MANIFEST_PATH "${CMAKE_CURRENT_LIST_DIR}/../Cargo.toml")

    if(NOT EXISTS "${CRATE_MANIFEST_PATH}")
        message(FATAL_ERROR "The path ${CMAKE_CURRENT_LIST_DIR} does not contain a Cargo.toml")
    else()
        message(STATUS "Importing crate CRATE_MANIFEST_PATH=${CRATE_MANIFEST_PATH}")
    endif()

    ## Simplyfy inputs
    set(_LIB_NAME ${_RUST_LIB_NAME})
    set(CXXBRIDGE_TARGET ${_RUST_LIB_NAME}_bridge)

    # install cxx before corrsoion setup to avoid reinstall cxx with cargo update
    execute_process(
        COMMAND cargo tree -i cxx --depth=0 --manifest-path ${CRATE_MANIFEST_PATH}
        RESULT_VARIABLE cxx_version_result
        OUTPUT_VARIABLE cxx_version_output
    )
    if(NOT "${cxx_version_result}" EQUAL "0")
        message(FATAL_ERROR "Crate ${_arg_CRATE} does not depend on cxx.")
    endif()
    if(cxx_version_output MATCHES "cxx v([0-9]+.[0-9]+.[0-9]+)")
        set(cxx_required_version "${CMAKE_MATCH_1}")
    else()
        message(FATAL_ERROR "Failed to parse cxx version from cargo tree output: `cxx_version_output`")
    endif()

    execute_process(
        COMMAND cargo install --locked cxxbridge-cmd --version ${cxx_required_version}
        RESULT_VARIABLE cxx_version_result
        OUTPUT_VARIABLE cxx_version_output
    )
    # TODO: disable using non system memory allocator until we can make it work with other jemalloc libraries such as pyarrow
    # use mimalloc for macOS so that we could avoid https://github.com/vectordotdev/vector/issues/14946 when using Rosetta
    # if(APPLE)
    #     message(STATUS "Use mimalloc memory allocator for vector under macOS")
    #     set(MEMORY_ALLOCATOR_FEATURE "vector/mimalloc")
    # else()
    #     message(STATUS "Use default memory allocator for vector under Linux")
    #     # tikv-jemallocator is the default memory allocator
    #     set(MEMORY_ALLOCATOR_FEATURE "")
    # endif()

    message(STATUS "ENV PROFILE_NAME is $ENV{PROFILE_NAME}")
    if(DEFINED ENV{PROFILE_NAME} AND NOT "$ENV{PROFILE_NAME}" STREQUAL "")
        set(VECTOR_PROFILE_NAME $ENV{PROFILE_NAME})
        message(STATUS "Use ${VECTOR_PROFILE_NAME} in environment var")
    else()
        set(VECTOR_PROFILE_NAME "")
        message(STATUS "No ${VECTOR_PROFILE_NAME} in environment var")
    endif()
    ## Import Rust target
    corrosion_import_crate(
        MANIFEST_PATH "${CRATE_MANIFEST_PATH}"
        FEATURES ${MEMORY_ALLOCATOR_FEATURE}
        LOCKED
        PROFILE ${VECTOR_PROFILE_NAME})

    corrosion_add_cxxbridge(${CXXBRIDGE_TARGET} CRATE ${_LIB_NAME} FILES lib.rs)
    set_property(TARGET ${CXXBRIDGE_TARGET} PROPERTY POSITION_INDEPENDENT_CODE ON)

    if(NOT DEFINED VCPKG_TARGET_TRIPLET)
        if(APPLE)
            set(VCPKG_TARGET_TRIPLET "x64-osx")
        else()
            set(VCPKG_TARGET_TRIPLET "x64-linux")
        endif()
        message(STATUS "set vcpkg target triplet VCPKG_TARGET_TRIPLET=${VCPKG_TARGET_TRIPLET}")
    endif()

    update_target_properties()

    install(TARGETS ${_LIB_NAME}
            EXPORT ${EXPORT_TARGET_NAME}
    )

    set_target_properties(${CXXBRIDGE_TARGET} PROPERTIES
            PUBLIC_HEADER "${CMAKE_CURRENT_BINARY_DIR}/corrosion_generated/cxxbridge/${CXXBRIDGE_TARGET}/include/${CXXBRIDGE_TARGET}/lib.h")

    install(TARGETS ${CXXBRIDGE_TARGET}
            EXPORT ${EXPORT_TARGET_NAME}
            PUBLIC_HEADER DESTINATION include/${CXXBRIDGE_TARGET}
    )

endfunction(add_library_rust)

if("${CMAKE_SYSTEM_NAME}" STREQUAL "Windows")
    set(Rust_CARGO_TARGET "x86_64-pc-windows-gnu")
elseif("${CMAKE_SYSTEM_NAME}" STREQUAL "Linux")
    execute_process(
        COMMAND uname -m
        RESULT_VARIABLE exit_code_or_error
        OUTPUT_VARIABLE OS_ARCHITECTURE
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    set(Rust_CARGO_TARGET "${OS_ARCHITECTURE}-unknown-linux-gnu")
elseif("${CMAKE_SYSTEM_NAME}" STREQUAL "Darwin")
    if("${CMAKE_OSX_ARCHITECTURES}" STREQUAL "x86_64")
        set(Rust_CARGO_TARGET "x86_64-apple-darwin")
    else()
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
   endif()
else()
    message(FATAL_ERROR "hardcoded ${CMAKE_SYSTEM_NAME} platformchecks not supported outside windows-gnu, linux-gnu and apple-darwin")
endif()

find_package(Corrosion REQUIRED)
add_library_rust(NAME vectorcxx NAMESPACE vectorcxx)
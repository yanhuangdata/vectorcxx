set(VCPKG_TARGET_ARCHITECTURE arm64)
set(VCPKG_CRT_LINKAGE dynamic)
set(VCPKG_LIBRARY_LINKAGE static)

set(VCPKG_CMAKE_SYSTEM_NAME Linux)

# set compiler track to true
# so that we can build dependencies with one compiler (e.g. gcc) and
# build the application with another compiler (e.g. clang)
set(VCPKG_DISABLE_COMPILER_TRACKING true)

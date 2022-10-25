#!/usr/bin/env just --justfile

set dotenv-load := true

build_os := if os() == "macos" { "osx" } else { "linux" }
generator := env_var_or_default("CMAKE_GENERATOR", "Unix Makefiles")
vcpkg_default_triplet := env_var_or_default("VCPKG_DEFAULT_TRIPLET", "x64-" + build_os + "-haswell")
default_build_type := env_var_or_default("BUILD_TYPE", "debug")
default_build_jobs := env_var_or_default("BUILD_JOBS", "12")
default_build_target := env_var_or_default("BUILD_TARGET", "all")
default_distcc_jobs := env_var_or_default("DISTCC_JOBS", "24")

# configure the project using cmake
cmake build_type=default_build_type:
  # set the OPENSSL_ROOT_DIR to the package installed via manifest mode,
  # so that `find_package(OpenSSL)` in CMake can find the OpenSSL lib with correct target architecture
  OPENSSL_ROOT_DIR=./build-{{build_type}}-{{build_os}}-x64-cmake/vcpkg_installed/{{vcpkg_default_triplet}} cmake . --preset={{build_type}}-{{build_os}} -DVCPKG_TARGET_TRIPLET={{vcpkg_default_triplet}}

# compile the project
build build_type=default_build_type jobs=default_build_jobs target=default_build_target:
	cmake --build --target {{target}} --preset={{build_type}}-{{build_os}}-build -j {{jobs}}

# run all tests
test build_type=default_build_type:
  pushd build-{{build_type}}-{{build_os}}-x64-cmake && make test && popd
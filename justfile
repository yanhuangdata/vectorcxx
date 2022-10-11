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
  cmake . --preset={{build_type}}-{{build_os}} -DVCPKG_TARGET_TRIPLET={{vcpkg_default_triplet}}

# compile the project
build build_type=default_build_type jobs=default_build_jobs target=default_build_target:
	cmake --build --target {{target}} --preset={{build_type}}-{{build_os}}-build -j {{jobs}}

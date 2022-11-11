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
  OPENSSL_ROOT_DIR=./build-cmake-{{build_type}}-{{build_os}}/vcpkg_installed/{{vcpkg_default_triplet}} cmake . --preset={{build_type}}-{{build_os}} -DVCPKG_TARGET_TRIPLET={{vcpkg_default_triplet}}

# compile the project
build build_type=default_build_type jobs=default_build_jobs target=default_build_target:
	cmake --build --target {{target}} --preset={{build_type}}-{{build_os}}-build -j {{jobs}}

# install the project
install build_type=default_build_type: 
	DESTDIR=./{{build_type}}-{{build_os}} cmake --build --target install --preset={{build_type}}-{{build_os}}-build 

# run all tests
test build_type=default_build_type:
  pushd build-cmake-{{build_type}}-{{build_os}} && ctest && popd

clean:
  rm -fr target && rm -fr vcpkg_installed && rm -fr build-cmake-*

# patch ./vector/Cargo.toml using ./patch/Cargo.patch.json
patch: 
  # @pushd vector && git diff --exit-code ./Cargo.toml > /dev/null || (echo "./vector/Cargo.toml has uncommitted modification, please commit the necessary changes or discard the changes manually, and then run 'patch' command again" && exit 1)
  # this will reset the ./vector/Cargo.toml to the original state
  cd vector && git checkout HEAD -- ./Cargo.toml && cd ..
  tomlpatch ./vector/Cargo.toml ./patch/Cargo.patch.json 
  @echo "Updating patched Cargo.toml for vector"
  cd vector && cargo update --package openssl --package rdkafka && cd ..
  cp ./vector/Cargo.toml ./patch/ && cp ./vector/Cargo.lock ./patch/

install_toml_patch:
  pip install tomlpatch --upgrade

# switch to x64_toolchain
x64_toolchain:
    rustup override set 1.64.0-x86_64-apple-darwin
    rustup override set 1.64.0-x86_64-apple-darwin --path vector
    rustup override list

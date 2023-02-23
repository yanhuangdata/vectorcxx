# vectorcxx
* This project is a C++ wrapper for [vector](https://vector.dev)

# how it works
* This project uses [cxx](https://cxx.rs) to call Rust code from C++, and uses [corrosion](https://github.com/corrosion-rs/corrosion) to integrate the library into CMake.

# build on Apple Silicon for arm64
* use `nightly-aarch64-apple-darwin` rust toolchain
* Export the following environment variables
```
VCPKG_DEFAULT_TRIPLET=arm64-osx
VCPKG_DEFAULT_HOST_TRIPLET=arm64-osx
```

# develop (on macOS)
* add submodule for vectorcxx
```
# only needed to be run for the first time
git submodule update --init --recursive
```
* configure and build
```
export VECTORCXX_VCPKG_MANIFEST_DIR=$PWD/ci/dev_image
```
```
just cmake
# this will use vcpkg manifest mode to install all the dependencies
just build
```
* patch vector Cargo.toml
```
# only needed to be installed for the first time user
# remember that tomlpatch requires python version >= 3.10.0
just install_toml_patch
just patch
```
Commit the `Cargo.toml` and `Cargo.lock` in the `patch` directory to the vectorcxx repo

* re-generate `CMakelists.txt`
Everytime `CMakelists.txt` is re-generated, the following changes need to be made (until we remove all of them out of it):
* `if ("${CMAKE_SYSTEM_NAME}" STREQUAL "Darwin")` branch for dependencies

## development x64 under Apple Silicon with Rosetta
1. Open terminal with Rosetta
2. Set toolchain
```
just x64_toolchain

# use direnv + .envrc is recommended for env var management
export CMAKE_OSX_ARCHITECTURES=x86_64
```
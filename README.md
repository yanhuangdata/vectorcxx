# vectorcxx
* This project is a C++ wrapper for [vector](https://vector.dev)

# how it works
* This project uses [cxx](https://cxx.rs) to call Rust code from C++, and uses [corrosion](https://github.com/AndrewGaspar/corrosion) to integrate the library into CMake.

# setup
* Rust `edition2021` is needed because vector requires it.
    * `rustup default nightly`

# build on Apple Silicon for arm64
* use `nightly-aarch64-apple-darwin` rust toolchain
* Export the following environment variables
```
VCPKG_DEFAULT_TRIPLET=arm64-osx
VCPKG_DEFAULT_HOST_TRIPLET=arm64-osx
```

# develop (on macOS)
* configure and build
```
just cmake
# this will use vcpkg manifest mode to install all the dependencies
just build
```
* patch vector Cargo.toml
```
# only needed to be installed for the first time user
just install_toml_patch
just patch
```
Commit the `Cargo.toml` and `Cargo.lock` in the `patch` directory to the vectorcxx repo
# vectorcxx
* This project is a C++ wrapper for [vector](https://vector.dev)

# how it works
* This project uses [cxx](https://cxx.rs) to call Rust code from C++, and uses [corrosion](https://github.com/AndrewGaspar/corrosion) to integrate the library into CMake.

# setup
* Rust `edition2021` is needed because vector requires it.
    * `rustup default nightly`
* vcpkg dependencies
  * `Corrosion`/`zlib`/`Catch`
  * `vcpkg install`

# develop (on macOS)
* configure and build
```
cmake . --preset=debug-osx
cmake --build --preset=debug-osx-build
```
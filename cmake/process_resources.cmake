# copy `Cargo.toml` and `Cargo.lock` from `./patch/` directory to `./vector` directory
function (patch_cargo_toml)
    file (COPY ${CMAKE_CURRENT_SOURCE_DIR}/patch/Cargo.toml DESTINATION ${CMAKE_CURRENT_SOURCE_DIR}/vector)
    file (COPY ${CMAKE_CURRENT_SOURCE_DIR}/patch/Cargo.lock DESTINATION ${CMAKE_CURRENT_SOURCE_DIR}/vector)
    message(STATUS "[copy patches] Patched Cargo.toml and Cargo.lock copied to 'vector' directory")
endfunction()

patch_cargo_toml()

include(cmake/rust_bridge.cmake)
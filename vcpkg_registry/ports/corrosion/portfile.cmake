vcpkg_from_github(
    OUT_SOURCE_PATH SOURCE_PATH
    REPO corrosion-rs/corrosion
    REF b6297a8630140ddadb040a4c979df85c76909aa1
    SHA512 eca296208c2c79ef238473a7774ffc3d564769ddf3fb67809fae7b97fce9865a97f0d02384b6d630836e58384fae00d483ef31111820fbe915ad496331305278
    HEAD_REF master
)

vcpkg_cmake_configure(
    SOURCE_PATH "${SOURCE_PATH}"
    OPTIONS
        -DCORROSION_BUILD_TESTS=OFF
)

vcpkg_cmake_install()

# corrosion is a cmake only port
set(VCPKG_POLICY_EMPTY_INCLUDE_FOLDER enabled)

vcpkg_cmake_config_fixup(PACKAGE_NAME Corrosion CONFIG_PATH lib/cmake/Corrosion)
file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/lib")
file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug")

# Handle copyright
vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE")

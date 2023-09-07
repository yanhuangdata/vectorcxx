vcpkg_from_github(
    OUT_SOURCE_PATH SOURCE_PATH
    REPO corrosion-rs/corrosion
    REF 19126776f239a6d7b94235a34d168f21b0910a95
    SHA512 a48599b7ecd61f6f3058b524873b1ebbe3f6cd87cc9f747c6f503abca5b0f9335dbc8374489baef04016d52fdd5870ef8a5c962faa9dd2d2b55670791bb67bd3
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

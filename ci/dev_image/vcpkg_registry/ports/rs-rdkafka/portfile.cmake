set(SOURCE_PATH ${CMAKE_CURRENT_LIST_DIR})

# rs-rdkafka is a cmake only port
set(VCPKG_POLICY_EMPTY_INCLUDE_FOLDER enabled)

# rust rdkafka only searches for `rdkafka` but not `rdkafka-static`
file(INSTALL ${CURRENT_PACKAGES_DIR}/../librdkafka_${TARGET_TRIPLET}/debug/lib/pkgconfig/rdkafka++-static.pc DESTINATION ${CURRENT_PACKAGES_DIR}/debug/lib/pkgconfig RENAME rdkafka.pc)
file(INSTALL ${CURRENT_PACKAGES_DIR}/../librdkafka_${TARGET_TRIPLET}/lib/pkgconfig/rdkafka++-static.pc DESTINATION ${CURRENT_PACKAGES_DIR}/lib/pkgconfig RENAME rdkafka.pc)

# Handle copyright
vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/LICENSE")

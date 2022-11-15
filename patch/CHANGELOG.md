# 2022-11-15
* Turning jemalloc off so that system memory allocator is used, and this avoids crash when pyarrow is used together 
# 2022-11-01
* Upgrade to the latest version of vector (0.25 snapshot), and use rdkafka 1.9.2
# 2022-10-31
* `features.enterprise` is set to `[]` because one of the feature in the `enterprise` feature set cause memory allocation issue and made the app to crash
* Pin `rdkafka` to 0.28.0 and `zstd` to `0.10.2` to aoid upgrading rdkafka to 1.9.2. Basically, we revert vector commit `ee3afe0a81d5bb20c3d8f6e6c8850a040265442d`. We should upgrade to 1.9.2 in the future. 

# 2022-10-29
* `dependencies.openssl.features` is set to `null` to remove `["vendered"]` feature so that linking to app with openssl won't cause trouble
* `dependencies.rdkafka.features` is set to `dynamic-linking` so that app can link to librdkafka dynamically
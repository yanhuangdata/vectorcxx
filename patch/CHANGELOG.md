# 2022-10-31
* `features.enterprise` is set to `[]` because one of the feature in the `enterprise` feature set cause memory allocation issue and made the app to crash
* Pin `rdkafka` to 0.28.0 to aoid upgrading rdkafka to 1.9.2. We should upgrade to 1.9.2 in the future.

# 2022-10-29
* `dependencies.openssl.features` is set to `null` to remove `["vendered"]` feature so that linking to app with openssl won't cause trouble
* `dependencies.rdkafka.features` is set to `dynamic-linking` so that app can link to librdkafka dynamically
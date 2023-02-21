FROM debian:bullseye-20221024 as vectorcxx_dev_image

RUN apt-get update \
    && apt-get --yes install \
    apt-utils \
    g++ \
    lld \
    cmake \
    ninja-build \
    git \
    curl \
    unzip \
    python3.9 \
    python3.9-distutils \
    python3.9-dev \
    python3-pip \
    python3-venv \
    clang-tidy-11 \
    clang-format-11 \
    clang-11 \
    llvm-11-dev \
    pkg-config \
    zip \
    ccache \
    autoconf \
    libtool \
    libarchive-tools \
    && apt-get clean

RUN ln -s /usr/bin/clang-format-11 /usr/bin/clang-format

# not yet needed?
# RUN pip3 install poetry

ARG CURL_DOWNLOAD="curl --retry 5 --retry-delay 10 --retry-max-time 60 -fsSL"

# install rust compiler and cargo
# 1.64.0 is consistent with rust-toolchain.toml in vector
ARG RUST_TOOLCHAIN_VERSION=1.64.0
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain ${RUST_TOOLCHAIN_VERSION} -y && \
    rm -rf /root/.rustup/toolchains/1.64.0-x86_64-unknown-linux-gnu/share/doc/rust
ARG SCCACHE_VERSION=v0.3.0
ARG SCCACHE_ARTIFACT=sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl
RUN ${CURL_DOWNLOAD} https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${SCCACHE_ARTIFACT}.tar.gz | bsdtar -xf- --strip-components 1 -C /usr/local/bin \
    && chmod +x /usr/local/bin/sccache
# for some reason, adding cargo bin to path doesn't work in CMakeLists.txt
RUN ln -s $HOME/.cargo/bin/cargo /usr/local/bin/cargo
RUN ln -s $HOME/.cargo/bin/rustc /usr/local/bin/rustc
RUN ln -s $HOME/.cargo/bin/rustup /usr/local/bin/rustup
# ENV RUSTC_WRAPPER=/usr/local/bin/sccache


ENV VCPKG_ROOT=/vcpkg
ENV HOME=/root
ENV PATH="$HOME/.cargo/bin:$PATH"
# https://github.com/rust-lang/cargo/issues/7515
ENV CARGO_HTTP_MULTIPLEXING=false
# using both git tag (`2020.01` for example) or commit id works
# This is added to avoid Docker cache the vcpkg repo and the build will fail reporting incorrect vcpkg baseline
ARG VCPKG_BASELINE=4340582bc30f3e539bd4f52e26418f7d62145956
ARG VCPKG_GIT_REPO=https://github.com/microsoft/vcpkg.git
# Set vcpkg binary caching folder, https://github.com/microsoft/vcpkg/blob/master/docs/users/binarycaching.md
ENV VCPKG_BINARY_SOURCES=clear;files,/vcpkg_cache,readwrite
ENV VCPKG_DEFAULT_TRIPLET=x64-linux-haswell
ENV VCPKG_DEFAULT_HOST_TRIPLET=x64-linux-haswell

WORKDIR /
RUN git clone ${VCPKG_GIT_REPO} && \
    cd vcpkg && \
    git checkout ${VCPKG_BASELINE}

WORKDIR /vcpkg
RUN ./bootstrap-vcpkg.sh

# Copy additional custom triplets which limit the arch to haswell and disable AVX512 instruction set
COPY vcpkg_registry/triplets/x64-linux-haswell.cmake /vcpkg/triplets/
COPY mirrors_cn/cargo_config.toml ${HOME}/.cargo/config

# link python
RUN ln -s /usr/bin/python3 /usr/bin/python

# install just as command runner
RUN curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin

# replace cmake with 3.24
RUN mkdir -p /root/downloads && \
    cd /root/downloads/ && \
    curl -L https://github.com/Kitware/CMake/releases/download/v3.24.3/cmake-3.24.3-linux-x86_64.tar.gz -o cmake-3.24.3.tar.gz && \
    tar xzvf cmake-3.24.3.tar.gz && \
    rm /usr/bin/cmake && \
    rm -rf /usr/share/cmake-3.18 && \
    mv /root/downloads/cmake-3.24.3-linux-x86_64/bin/cmake /usr/bin/ && \
    mv /root/downloads/cmake-3.24.3-linux-x86_64/share/cmake-3.24 /usr/share/ && \
    rm -rf /root/downloads

RUN pwd && ls
#install vcpkg dependencies
COPY vcpkg.json /vcpkg/
COPY vcpkg-configuration.json /vcpkg/
COPY vcpkg_registry /vcpkg/vcpkg_registry
RUN /vcpkg/vcpkg install && \
    rm -fr ${VCPKG_ROOT}/buildtrees && rm -fr ${VCPKG_ROOT}/packages && rm -fr ${VCPKG_ROOT}/vcpkg_installed

#remove buildtrees
# RUN rm -fr ${VCPKG_ROOT}/buildtrees && rm -fr ${VCPKG_ROOT}/packages && rm -fr ${VCPKG_ROOT}/vcpkg_installed


CMD ["/bin/bash"]

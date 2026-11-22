# dev-env-docker: Pin base image versions; dev-env-ubuntu-snapshot: signed snapshots.
FROM ubuntu:24.04@sha256:73b999393d07f2bf65a40373778a44dc9a6d54ad7718d29247d8ff9413c4d6a8
ENV DEBIAN_FRONTEND=noninteractive \
    TMPDIR=/workspaces/.cache/tmp \
    PANEL_DEV_CONTAINER=1 \
    COREPACK_HOME=/workspaces/.cache/corepack \
    NPM_CONFIG_CACHE=/workspaces/.cache/npm \
    MISE_DATA_DIR=/opt/mise-data \
    PATH=/opt/mise-data/shims:/usr/local/bin:/usr/bin:/bin
RUN mkdir -p "$TMPDIR" \
    && apt-get -o APT::Update::Error-Mode=any update \
    && apt-get install -y --no-install-recommends ca-certificates=20240203 openssl=3.0.13-0ubuntu3 \
    && sed -i 's|http://archive.ubuntu.com/ubuntu/|https://snapshot.ubuntu.com/ubuntu/20261116T000000Z/|g; s|http://security.ubuntu.com/ubuntu/|https://snapshot.ubuntu.com/ubuntu/20261116T000000Z/|g' /etc/apt/sources.list.d/ubuntu.sources
COPY .devcontainer/apt-packages.lock /opt/apt-packages.lock
RUN apt-get -o APT::Update::Error-Mode=any update \
    && sed '/^#/d; /^$/d' /opt/apt-packages.lock | xargs apt-get install -y --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*
# dev-env-mise-shasums: archive checksum; dev-env-mise: exact tools; the installer is fetched at its immutable release tag.
ADD --checksum=sha256:23ba592fcbc61f057e2b82c1eafd085832712ba0c759b714ca16cd06babca378 https://github.com/jdx/mise/releases/download/v2026.11.9/mise-v2026.11.9-linux-x64.tar.gz /opt/mise.tar.gz
RUN tar -xzf /opt/mise.tar.gz -C /opt \
    && install /opt/mise/bin/mise /usr/local/bin/mise \
    && rm /opt/mise.tar.gz
COPY mise.toml rust-toolchain.toml /opt/toolchain/
WORKDIR /opt/toolchain
RUN mise trust && mise install && mise reshim
WORKDIR /workspaces/panel

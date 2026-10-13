# Rust + Tauri v2 build/test environment for Android Stream Desk.
#
# Purpose: compile and unit-test the Rust side (and optionally produce the
# Linux .deb / .AppImage) without installing a Rust toolchain or the
# webkit2gtk/GTK dev headers on the host. The host only needs Docker.
#
# Debian bookworm's glibc (2.36) is older than Ubuntu 24.04's, so binaries
# built here run on the host and on most current distros.
#
# Build via scripts/rust-env.sh (it passes your UID/GID so files written into
# the bind-mounted repo are owned by you, not root).
FROM rust:1-bookworm

ARG UID=1000
ARG GID=1000
ENV DEBIAN_FRONTEND=noninteractive

# Tauri v2 Linux prerequisites + tooling used by the bundler.
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl wget file git pkg-config patchelf xz-utils \
      libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
      librsvg2-dev libxdo-dev \
    && rm -rf /var/lib/apt/lists/*

# Node 22 + pnpm so `pnpm tauri build` can run its beforeBuildCommand inside
# the container too. pnpm is installed directly (corepack's shim fails to
# fetch current pnpm releases under Node 22) and pinned to the same major as
# CI (pnpm-workspace.yaml uses pnpm-10 keys).
#
# The image is always built for linux/amd64 (scripts/rust-env.sh passes
# --platform), on Apple Silicon too: the Android image layered on top needs
# x86_64 host binaries (NDK), so the x64 Node tarball is the right one here.
# Bump NODE_SHA256 together with NODE_VERSION, from
# https://nodejs.org/dist/v<version>/SHASUMS256.txt (linux-x64.tar.xz line).
ARG NODE_VERSION=22.20.0
ARG NODE_SHA256=00bbd05e306ea68b6e13e17360d0e2f680b493ef95f2fea1c4296ff7437530bc
ARG PNPM_VERSION=10.34.6
RUN curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" -o /tmp/node.tar.xz \
    && echo "${NODE_SHA256}  /tmp/node.tar.xz" | sha256sum -c - \
    && tar -xJf /tmp/node.tar.xz -C /usr/local --strip-components=1 \
    && rm /tmp/node.tar.xz \
    && npm install -g "pnpm@${PNPM_VERSION}" \
    && npm cache clean --force

# Non-root user matching the host user so bind-mounted files stay ours.
RUN (getent group "${GID}" >/dev/null || groupadd -g "${GID}" dev) \
    && useradd -m -u "${UID}" -g "${GID}" -s /bin/bash dev

# Cargo registry/git cache lives in the user's home so a named volume can
# persist it across runs. Toolchain stays in the image (/usr/local/rustup).
ENV CARGO_HOME=/home/dev/.cargo \
    CARGO_TERM_COLOR=always \
    # AppImage tooling needs FUSE unless told to self-extract.
    APPIMAGE_EXTRACT_AND_RUN=1
RUN mkdir -p /home/dev/.cargo && chown -R dev:"${GID}" /home/dev

# Container-only node_modules: scripts/rust-env.sh mounts a named volume here,
# over the bind-mounted repo, so pnpm inside the container never touches the
# host's node_modules (which holds darwin/win32 native binaries on non-Linux
# hosts). Pre-created owned by dev so the volume initialises writable. The
# pnpm store lives inside it to keep hardlinks on one filesystem.
RUN mkdir -p /work/node_modules && chown dev:"${GID}" /work /work/node_modules
ENV npm_config_store_dir=/work/node_modules/.pnpm-store

USER dev
WORKDIR /work
CMD ["bash"]

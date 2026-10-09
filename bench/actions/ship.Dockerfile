# bench/actions/ship.Dockerfile: the ship's images, the build environment of the release workflow's Linux jobs
# (.github/workflows/release.yml; docs/DEVELOPING.md, "Releases"), linux/amd64's and linux/arm64's: Ubuntu 26.04
# by the digest of its index (which holds both platforms' images) and bench/actions/toolchain.sh's install for
# the machine's architecture, every package and tool at a pinned version. bench/actions/image.sh (or Depot's
# builder, ship-image.yml) builds them from the checkout's root, whose context ship.Dockerfile.dockerignore keeps
# to this file, toolchain.sh, bench/zig.sh and bench/cross-ship.sh, and pushes them; the workflow pulls each by
# the digest bench/actions/ship-image.txt names for its architecture.
FROM ubuntu:26.04@sha256:f144425ff09be612d6d9ad965196e9cdc23dae1f42110a8a11a3e9a8198759f7
COPY bench/actions/ship.Dockerfile bench/actions/toolchain.sh bench/zig.sh bench/cross-ship.sh /opt/teq-ship/recipe/
RUN /opt/teq-ship/recipe/toolchain.sh install /opt/teq-ship/recipe
# RUSTUP_TOOLCHAIN is toolchain.sh's $rust: rustup prefers it to rust-toolchain.toml's floating channel.
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo RUSTUP_TOOLCHAIN=1.98.1 TEQ_SHIP_CACHE=/opt/teq-ship/cache \
    PATH=/opt/cargo/bin:/opt/node/bin:/opt/sbt/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    LANG=C.UTF-8
LABEL org.opencontainers.image.source=https://github.com/Carrot-Inc/teq

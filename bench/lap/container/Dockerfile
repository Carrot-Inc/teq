# An arm64 Linux on a Mac for instruction counts by function: valgrind has no macOS arm64 port,
# and cachegrind here counts what the host's `/usr/bin/time -l` counts to 0.1 or 0.2 points
# of a program's delta (the kernel's share and Darwin's thread-local calls apart).
FROM rust:1.98.1-slim-trixie
RUN apt-get update && apt-get install -y --no-install-recommends valgrind python3 binutils && rm -rf /var/lib/apt/lists/*

# bench/ship-metadata.sh, sourced by bench/cross-ship.sh and bench/pgo.sh ship from the root of the tree: teq's
# crate metadata (`-C metadata`), which names teq's functions in a profile: the crate's id in every name, and
# the module of each local one. A guided build takes a profile's records only under the metadata of the
# instrumented build that made it, which its training records (`training metadata`, bench/ship-manifest.sh), and
# cargo derives another for another version, host or target. metadata_wrapper <file>: a RUSTC_WRAPPER there that
# gives teq's crate -C metadata=$TEQ_CROSS_METADATA and every other crate cargo's own, appending teq's command
# to $TEQ_CROSS_LOG.rustc when TEQ_CROSS_LOG is set; with TEQ_CROSS_PROBE=<file> it writes the metadata cargo
# gave teq there and stops the build. metadata_probe <wrapper> <dir>: the metadata cargo gives teq's ship build
# here (no --target: the host's), read by a build under <dir> that stops at teq's crate; no linker reaches it.
metadata_wrapper() {
  cat > "$1.new" <<'WRAPPER'
#!/bin/bash
# Written by bench/ship-metadata.sh: RUSTC_WRAPPER giving teq's crate -C metadata=$TEQ_CROSS_METADATA.
rustc=$1
shift
teq=
prev=
for a in "$@"; do [ "$prev" = --crate-name ] && [ "$a" = teq ] && teq=1; prev=$a; done
[ -n "$teq" ] || exec "$rustc" "$@"
[ -n "$TEQ_CROSS_PROBE" ] && { prev=; for a in "$@"; do [ "$prev" = -C ] && [[ $a == metadata=* ]] && echo "${a#metadata=}" > "$TEQ_CROSS_PROBE"; prev=$a; done; exit 1; }
args=()
prev=
for a in "$@"; do
  [ "$prev" = -C ] && [[ $a == metadata=* ]] && a=metadata=$TEQ_CROSS_METADATA
  args+=("$a")
  prev=$a
done
[ -z "$TEQ_CROSS_LOG" ] || echo "$rustc ${args[*]}" >> "$TEQ_CROSS_LOG.rustc"
exec "$rustc" "${args[@]}"
WRAPPER
  chmod +x "$1.new" && mv "$1.new" "$1"
}

metadata_probe() {
  local value
  rm -f "$2/metadata"
  mkdir -p "$2" || return 1
  env -u RUSTFLAGS -u CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER="$1" TEQ_CROSS_PROBE="$2/metadata" timeout 300 cargo build --profile ship --target-dir "$2/probe" > /dev/null 2>&1
  value=$(cat "$2/metadata" 2> /dev/null)
  [[ $value =~ ^[0-9a-f]+$ ]] || { echo "no metadata read for teq's native build" >&2; return 1; }
  echo "$value"
}

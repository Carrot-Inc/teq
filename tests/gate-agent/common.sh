# Sourced by the stand-ins of the remote machines' scripts tests/gate.sh drives through REMOTE_AGENT, for
# tests/gate-scripts.sh: each machine a directory $GATE_AGENT_ROOT/machines/<name> (its home, the projects mirrored
# under rw/), the pool $GATE_AGENT_ROOT/pool (a line a machine: its name and its words, `holds=` among them; a
# claimed machine shows as busy), a claim a file under $GATE_AGENT_ROOT/claims. A command runs in the project's
# mirror with HOME the machine's directory, its env file sourced (its login environment) and the stand-ins of cargo
# and rustc of $GATE_AGENT_ROOT/bin first on the PATH. GATE_AGENT_CORRUPT=<machine> spoils the binary mirrored to
# that machine as teq-head.
[ -n "$GATE_AGENT_ROOT" ] || { echo "GATE_AGENT_ROOT names no stand-in pool" >&2; exit 2; }
machine_of() { echo "${1%%/*}"; }
claimed() { [ -e "$GATE_AGENT_ROOT/claims/$1" ] && [ "$(cat "$GATE_AGENT_ROOT/claims/$1")" = "$2" ]; }

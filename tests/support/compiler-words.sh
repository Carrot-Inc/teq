# The spelling of the compiler's verbs by binary, for the scripts that run two binaries, one of which may be the
# reference's or master's from before the project verbs took the top level (2026-10): `compiler_words
# <bin>` answers the words to put before build, check or watch (`compiler`, or nothing under the old spelling),
# `interp_words <bin>` the verb that runs the interpreter (`interp`, or `run --target interp`). A binary's spelling
# is probed with `<bin> compiler --help` (exit 0 under the new spelling, 2 under the old), outside every
# measurement. `spellings <bin>...` probes each binary once in the shell that calls it and exports what it found,
# so that compiler_words and interp_words, called after it in command substitutions or in the workers the script
# starts (which inherit the export), answer without starting the binary again: a script calls `spellings` with
# its binaries before its loops. A binary `spellings` was not given is probed at each call, since what a command
# substitution's subshell learns is gone with it. All three go, with their uses, when bench/reference.txt advances
# past the first release that carries the top-level verbs.
_spellings_nl='
'
spellings() {
  local bin which
  for bin in "$@"; do
    case "$_spellings_nl${_spellings-}" in *"$_spellings_nl$bin=new$_spellings_nl"* | *"$_spellings_nl$bin=old$_spellings_nl"*) continue ;; esac
    which=old
    "$bin" compiler --help > /dev/null 2>&1 && which=new
    _spellings="${_spellings-}$bin=$which$_spellings_nl"
  done
  export _spellings _spellings_nl
}
_new_spelling() {
  case "$_spellings_nl${_spellings-}" in
    *"$_spellings_nl$1=new$_spellings_nl"*) return 0 ;;
    *"$_spellings_nl$1=old$_spellings_nl"*) return 1 ;;
  esac
  "$1" compiler --help > /dev/null 2>&1
}
compiler_words() { if _new_spelling "$1"; then echo compiler; fi; }
interp_words() { if _new_spelling "$1"; then echo interp; else echo "run --target interp"; fi; }

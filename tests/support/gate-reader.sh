# Sourced from the repository's root: gate_reader <workspace> compiles scalac 3.8.4's reader of pickles
# (tests/tasty/fromtasty/Gate.scala) once into <workspace>, a directory of the caller's own, and prints its class
# path, scala-cli's messages on stderr; the caller runs it as `java -cp <class path> gate <jobs>`, as many at once as
# it likes. The workspace is named because scala-cli builds an absolute source under the source's own directory
# otherwise, whatever the directory it runs in, so that every launch of the reader on the machine would compile into
# one place and read from it.
gate_reader() {
  local cp
  cp=$(COURSIER_MODE=offline timeout 300 scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline -q \
    --workspace "$1" --print-class-path "$PWD/tests/tasty/fromtasty/Gate.scala" --dep org.scala-lang:scala3-compiler_3:3.8.4) || return 1
  echo "$cp" | tail -1
}

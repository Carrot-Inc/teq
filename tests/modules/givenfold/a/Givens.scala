package gfa

// A given of a literal: its own module and a downstream call it, as scalac does; the interpreter's
// fold of a pure library expression leaves another module's products alone, as the whole build
// leaves the program's.
object Givens:
  given Int = 1

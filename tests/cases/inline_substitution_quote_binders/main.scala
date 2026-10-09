// A local a stored body's quote defines has its declared type in the copy's terms, as its trees
// have: `val y: x.type = x` for the argument `O` is a `y: O.type`, which reflection reads through
// `ValDef.tpt` and finds a `Marker`. scalac prints the lines of the .expected file.
@main def run(): Unit = println(M.check(O))

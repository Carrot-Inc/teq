// A block's tuple pattern definition over a value not a tuple literal is
// dotty's `MatchableTuple` (Desugar.makePatDef, Desugar.scala 1595-1599,
// 1632-1645): the match binds the whole value, `case $1 @ (a, b) => $1`,
// and the holder `$2` holds it. Pinned to M10 (the tree's shape);
// the holders' numbers are dotty's (the macro masks the numbers).
def pair(): (Int, Int) = (1, 2)
@main def run(): Unit =
  println(Shape.names { val (a, b) = pair(); a + b })

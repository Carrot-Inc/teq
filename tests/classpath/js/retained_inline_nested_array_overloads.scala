// jars: fixtures
// targets: js interp jvm
// Two inline overrides told apart by their nested arrays' elements only (`int[][]` and
// `String[][]` once erased) keep a retained body each.
import fix.retain.{Grid, Grids}

@main def main(): Unit =
  val g: Grids = Grid
  println(g.cells(Array(Array(1, 2), Array(3))))
  println(g.cells(Array(Array("a"), Array("b", "c"))))

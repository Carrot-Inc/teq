// jars: fixtures
// targets: js interp jvm
// Two inline overrides told apart by their arrays' elements only (`int[]` and `String[]` once
// erased) keep a retained body each.
import fix.retain.{Sized, Sizes}

@main def main(): Unit =
  val z: Sizes = Sized
  println(z.size(Array(1, 2)))
  println(z.size(Array("a", "b")))

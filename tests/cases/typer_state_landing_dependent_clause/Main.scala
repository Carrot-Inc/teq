// A dependent leading clause (`using z: Z[x.type]`) takes the given the probe resolved for it,
// the probe putting the selected `x`'s path into the target as the application does: each
// transparent given expands once (121, 3), as scalac prints.
class C
object C:
  extension (c: C) def f(k: Int): Int = -k
extension (c: C)(using x: X)
  def f(using z: Z[x.type])(k: Int): Int =
    x.v * 100 + z.v * 10 + k
@main def run(): Unit =
  println(C().f(1))
  println(Counter.next)

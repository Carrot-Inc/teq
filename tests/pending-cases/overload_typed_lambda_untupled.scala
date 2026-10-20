// Pinned for the queued part inlining-lila-folds (the gate of 5010f41f8 and the review's finding 1: the typing of a
// typed function literal cached after its adaptation to the first alternative, `pretype_typed_lambda`, broke
// tests/scala3's boehm-berarducci and pos/i12211, pos/i3945; reverted). scalac accepts and prints the expected
// output; teq rejects the overloaded `map` with the typed, untupled literal. The port is `FunProto.typedArg`'s
// unadapted typing cached once and adapted per candidate with `normArg`'s untupling.
// A function literal whose parameters are all typed, passed to an overloaded method, is typed
// against the first alternative's formal, the next alternative's where that typing errs
// (`FunProto.cacheTypedArg`), and untupled where the formal takes a function of one tuple
// (`Applications.ptIsCorrectProduct`), as scalac types `m.map((k: K, v: V) => ...)` over a `Map`;
// a literal with an untyped parameter is typed ahead against the alternatives' common formal.
object Test:
  val m: Map[String, Int] = Map("a" -> 1, "b" -> 2)
  def main(args: Array[String]): Unit =
    val m1 = m.map((k: String, v: Int) => (k.toUpperCase, v + 1))
    println(m1)
    val m2 = m.map((k: String, v: Int) => v)
    println(m2.toList.sorted)
    val m3 = m.map((k: String, v) => k + v)
    println(m3.toList.sorted)
    val l = List((1, "x"), (2, "y")).map((n: Int, s: String) => s * n)
    println(l)
    println(Over.f((x: Int, y: Int) => x + y))
object Over:
  def f(s: String): String = s
  def f(g: ((Int, Int)) => Int): Int = g((2, 3))

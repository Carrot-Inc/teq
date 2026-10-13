// A local `inline given` is an inline method (dotty's `Parsers.givenDef`): a lambda that summons it,
// or names it, holds its expansion, which reads and writes the captured variables at each call.
@main def run(): Unit =
  var n = 0
  val base = 10
  inline given Int = { n += 1; base + n }
  val f = () => summon[Int]
  println(f())
  println(f())
  var m = 0
  inline given x: Long = { m += 1; m.toLong }
  println(x); println(x)
  val g = () => x
  println(g()); println(g())

// A local right-associative extension called with its own default left out, beside operands of each purity: a
// literal, a pure def's call, a warm object, an operand that only looks impure. The left operand of `+::` is bound to
// `x`, as scalac binds it (the JVM bound it to the default's parameter, which an operator whose operands are
// interchangeable could not show: `x + y` beside `x * 10 + y`).
object W { Log.add("init W"); val v = 6 }

object Log {
  var entries = List.empty[String]
  def add(s: String): Unit = entries ::= s
  def show(r: Int): Unit = { println(s"${entries.reverse.mkString(",")} $r"); entries = Nil }
}

object Main {
  def e(): Int = { Log.add("effect"); 1 }
  def pure(): Int = 7
  def dflt(): Int = { Log.add("D"); 2 }

  def main(args: Array[String]): Unit = {
    extension (x: Int) def +:(y: Int = 2): Int = x + y
    extension (x: Int) def +::(y: Int = 2): Int = x * 10 + y
    extension (x: Int) def -::(y: Int = pure()): Int = x * 10 + y
    extension (x: Int) def *::(y: Int = W.v): Int = x * 10 + y
    extension (x: Int) def /::(y: Int = dflt()): Int = x * 10 + y
    Log.show(+:()(1))
    Log.show(+::()(1))
    Log.show(+::()(e()))
    Log.show(-::()(e()))
    Log.show(W.v)
    Log.show(*::()(e()))
    Log.show(/::()(pure()))
  }
}

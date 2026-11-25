// A local right-associative extension's operands bound as scalac binds them, the left operand of `a +:: b` to `x` (the
// JVM bound it to `y`, which an operator whose operands are interchangeable could not show: `x + y` beside
// `x * 10 + y`), and an operand that reads an object already initialised beside one that runs code.
object O { Log.add("init O"); val v = 2 }

object Log {
  var entries = List.empty[String]
  def add(s: String): Unit = entries ::= s
  def show(r: Int): Unit = { println(s"${entries.reverse.mkString(",")} $r"); entries = Nil }
}

object Main {
  def e(): Int = { Log.add("effect"); 1 }

  def main(args: Array[String]): Unit = {
    extension (x: Int) def +:(y: Int): Int = x + y
    extension (x: Int) def +::(y: Int): Int = x * 10 + y
    Log.show(1 +:: 4)
    Log.show(O.v)
    Log.show(e() +: O.v)
    Log.show(e() +:: O.v)
  }
}

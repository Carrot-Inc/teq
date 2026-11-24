// A right-associative extension of a value class whose companion holds a lazy val alone (its first load runs nothing
// a program sees), called infix with operands that run code; and one whose companion prints when first loaded, called
// written out with two literal nulls. Both are written and run as scalac runs them.
class V(val n: Int) extends AnyVal {
  extension (x: Int) def +:(y: Int): Int = x + y + n
}
object V { lazy val unused: Int = { println("lazy"); 1 } }

class W(val n: Int) extends AnyVal {
  extension (x: String) def +:(y: String): String = x + "/" + y + n
}
object W { println("init") }

object Main {
  def num(n: Int): Int = { println("operand " + n); n }
  def main(args: Array[String]): Unit = {
    val v = new V(7)
    import v.*
    println(num(1) +: num(2))
    val w = new W(7)
    println(w.+:(null)(null))
  }
}

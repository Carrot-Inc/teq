// A right-associative extension of a value class whose companion logs when first loaded, its two operands' types
// erasing differently: called infix with a left operand that runs code, and written out with arguments that run code.
// scalac evaluates the infix call's left operand before the companion's first load and the written-out call's
// arguments after it.
class V(val n: Int) extends AnyVal {
  extension (x: Int) def +:(y: String): String = y + x + n
}
object V { Main.log = Main.log :+ "v" }

class W(val n: Int) extends AnyVal {
  extension (x: Int) def +:(y: String): String = y + x + n
}
object W { Main.log = Main.log :+ "w" }

object Main {
  var log = List.empty[String]
  def num(k: Int): Int = { log = log :+ ("n" + k); k }
  def note(s: String): String = { log = log :+ s; s }
  def infix(): Unit = {
    val v = new V(7)
    log = Nil
    import v.*
    val res: String = num(1) +: "a"
    println("infix " + res + " " + log.mkString(","))
  }
  def written(): Unit = {
    val w = new W(8)
    log = Nil
    val res: String = w.+:(note("b"))(num(2))
    println("written " + res + " " + log.mkString(","))
  }
  def main(args: Array[String]): Unit = {
    infix()
    written()
  }
}

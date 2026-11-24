// A right-associative extension of a value class whose companion logs when first loaded, called infix with an operand
// that reads an object with initialisers: scalac evaluates the left operand before the companion's first load (it
// lifts an operand that is no pure path), and the right one after it.
class V(val n: Int) extends AnyVal {
  extension (x: Int) def +:(y: Int): Int = x + y + n
}
object V { Main.log = Main.log :+ "v" }

class W(val n: Int) extends AnyVal {
  extension (x: Int) def +:(y: Int): Int = x + y + n
}
object W { Main.log = Main.log :+ "w" }

object Loaded { Main.log = Main.log :+ "l"; val k: Int = 2 }
object Fresh { Main.log = Main.log :+ "f"; val k: Int = 1 }

object Main {
  var log = List.empty[String]
  def num(i: Int): Int = { log = log :+ ("n" + i); i }
  def left(): Unit = {
    val v = new V(7)
    val pre = Loaded.k
    log = Nil
    import v.*
    val res = num(1) +: Loaded.k
    println("call left " + res + " " + log.mkString(","))
  }
  def right(): Unit = {
    val w = new W(7)
    log = Nil
    import w.*
    val res = Fresh.k +: num(2)
    println("object left " + res + " " + log.mkString(","))
  }
  def main(args: Array[String]): Unit = {
    left()
    right()
  }
}

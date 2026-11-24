// A right-associative extension of a value class whose companion logs when first loaded, written out, its method
// clause's argument a path into an object with initialisers (loaded already) and its receiver clause's a call that
// logs: scalac loads the companion, then evaluates the arguments as they are written.
class V(val n: Int) extends AnyVal {
  extension (x: Int) def +:(y: Int): String = "" + x + "|" + y + "|" + n
}
object V { Main.log = Main.log :+ "v" }

object Main {
  var log = List.empty[String]
  val k: Int = 2
  def num(i: Int): Int = { log = log :+ ("n" + i); i }
  def main(args: Array[String]): Unit = {
    val v = new V(7)
    log = Nil
    val res = v.+:(Main.k)(num(1))
    println(res + " " + log.mkString(","))
  }
}

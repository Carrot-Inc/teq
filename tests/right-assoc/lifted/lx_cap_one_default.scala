// lx_cap_one_default: a local right-associative extension def whose right parameter has a default.
object Main {
  var log = List.empty[String]
  def num(k: Int): Int = { log = log :+ ("n" + k); k }
  def note(s: String): String = { log = log :+ s; s }
  val field = 1
  def c_default_left(): Unit = {
    log = Nil
    val cap = field + 6
    extension (x: Int) def +:(y: String = note("d")): String = y + x + cap
    log = Nil
    val res: String = +:()(num(1))
    println("c_default_left " + res + " " + log.mkString(","))
  }
  def c_default_given(): Unit = {
    log = Nil
    val cap = field + 6
    extension (x: Int) def +:(y: String = note("d")): String = y + x + cap
    log = Nil
    val res: String = +:(note("a"))(num(1))
    println("c_default_given " + res + " " + log.mkString(","))
  }
  def c_infix_given(): Unit = {
    log = Nil
    val cap = field + 6
    extension (x: Int) def +:(y: String = note("d")): String = y + x + cap
    log = Nil
    val res: String = num(1) +: note("a")
    println("c_infix_given " + res + " " + log.mkString(","))
  }
  def main(args: Array[String]): Unit = {
    try c_default_left() catch { case e: Throwable => println("c_default_left !" + e.getClass.getName) }
    try c_default_given() catch { case e: Throwable => println("c_default_given !" + e.getClass.getName) }
    try c_infix_given() catch { case e: Throwable => println("c_infix_given !" + e.getClass.getName) }
  }
}

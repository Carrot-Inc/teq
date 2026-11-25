// A local trait's right-associative member and a local right-associative extension, each called written out in its
// compiled clause order with a lazy val whose initialiser logs as the first-written operand, beside an operand that
// logs: scalac reads the lazy val first, in the order written. teq's typer leaves the lazy val's read unbound
// (`is_stable` takes a lazy local as stable) and lists the operands in the declared clause order, which the back ends
// evaluate: the other order (pending, the typer's row).
object Main {
  var log = List.empty[String]
  def eff[V](s: String, v: V): V = { log = log :+ s; v }

  def member(): String = {
    val cap = 7
    lazy val lz: Int = eff("Z", 5)
    trait T { extension (x: Int) def +::(y: Int): String = s"x=$x y=$y c=$cap" }
    val t = new T {}
    t.+::(lz)(eff("L", 1))
  }

  def local(): String = {
    val cap = 7
    lazy val lz: Int = eff("Z", 5)
    extension (x: Int) def +::(y: Int): String = s"x=$x y=$y c=$cap"
    +::(lz)(eff("L", 1))
  }

  def main(args: Array[String]): Unit = {
    println(member() + " " + log.mkString(","))
    log = Nil
    println(local() + " " + log.mkString(","))
  }
}

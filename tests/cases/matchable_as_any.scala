// `Matchable` resolves in sources as it does from a jar: as `Any`, so an ascription,
// a bound and a match over it type as scalac types them.
object Main:
  def show[T <: Matchable](t: T): String = t match { case i: Int => s"int $i"; case other => s"other $other" }
  def main(args: Array[String]): Unit =
    val m: Matchable = 1
    val s: Matchable = "s"
    println(m match { case i: Int => i + 1; case _ => 0 })
    println(s)
    println(show(2))
    println(show("x"))

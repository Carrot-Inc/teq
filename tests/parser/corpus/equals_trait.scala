// `scala.Equals`, the parent of `Product`: a case class, a case object and a tuple conform to it,
// and `canEqual` answers through it as through the product itself.
final case class P(a: Int, b: String)
case object Q

object Main:
  def describe(e: Equals, other: Any): String = s"${e.canEqual(other)}"
  def main(args: Array[String]): Unit =
    val e: Equals = P(1, "x")
    println(e.canEqual(P(2, "y")))
    println(P(1, "x").canEqual("s"))
    println(describe(Q, Q))
    println(describe((1, 2), (3, 4)))
    println(describe(P(1, "x"), Q))
    val es: List[Equals] = List(P(1, "a"), Q, (1, "b"))
    println(es.size)
    val p: Product = P(3, "z")
    val pe: Equals = p
    println(pe.canEqual(p))

// jars: scala-library cats-kernel cats-core
// Checks with no error: cats' syntax through its implicit conversions (`catsSyntaxSemigroup`,
// `catsSyntaxEq`, `toShow`, each an `implicit def` with an implicit clause read from TASTy),
// `toTraverseOps`, whose result is the refinement `Ops[F, A] { type TypeClassType = Traverse[F] }`
// and whose `traverse` goes through that member, and Predef's conversions of scala-library
// (`int2bigDecimal` adapting an argument, `augmentString` and `intWrapper` giving members).
import cats.syntax.all._
object Main:
  def main(args: Array[String]): Unit =
    println(1 |+| 2)
    println("a" === "a")
    println(List(1, 2).show)
    val traversed: Option[List[Int]] = List(1, 2).traverse(x => Option(x))
    println(traversed)
    val d: BigDecimal = BigDecimal(2)
    println(d * 3)
    println("abc".head)
    println("abc".reverse)
    println(1 to 5)
    println((1 to 5).sum)
    println(3.max(4))

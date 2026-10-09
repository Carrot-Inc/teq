// jars: scala-library
// std: scala-library
// `summon[Mirror.ProductOf[P]]` in link mode: scala-library's `summon` is inline and gives
// `x.type`, a path to the mirror the typer holds in a val, so that val is declared with the
// mirror's refined type and `fromProduct` gives a `P`, not the `Any` of the val's class.
import scala.deriving.Mirror
import scala.compiletime.*

final case class P(a: Int, b: String)
final case class Q(p: P, n: Long)

object Main:
  inline def labels[T](using m: Mirror.ProductOf[T]): List[String] =
    constValueTuple[m.MirroredElemLabels].toList.map(_.toString)
  def build[T](values: Tuple)(using m: Mirror.ProductOf[T]): T = m.fromProduct(values)
  def viaDeclared(m: Mirror.ProductOf[P]): P = m.fromProduct((1, "x"))
  def main(args: Array[String]): Unit =
    val m = summon[Mirror.ProductOf[P]]
    val p: P = m.fromProduct((1, "x"))
    println(p)
    println(labels[P])
    println(build[Q]((p, 2L)))
    val mq = summon[Mirror.ProductOf[Q]]
    val q: Q = mq.fromProduct((P(3, "y"), 4L))
    println(q.p.b + q.n)
    println(summon[Mirror.ProductOf[P]].fromProduct((5, "z")).a)
    println(viaDeclared(summon[Mirror.ProductOf[P]]))
    val e: m.MirroredElemTypes = (6, "w")
    println(e)

// jars: scala-library cats-kernel cats-core
// `collect` on a map with a lambda whose body is a match, cats' syntax in scope: the lambda
// applies to the map's own `collect`, and no enrichment of cats' is tried in its place.
import cats.syntax.all.*
case class P(price: Int)
object Main:
  def main(args: Array[String]): Unit =
    val m: Map[Int, P] = Map(1 -> P(10), 2 -> P(20), 3 -> P(30))
    val r: Map[Int, Int] = m.collect(it => it._2.price match { case n if n > 15 => it._1 -> n })
    println(r)
    println(m.collect(it => it._2.price match { case n if n > 15 => n }))

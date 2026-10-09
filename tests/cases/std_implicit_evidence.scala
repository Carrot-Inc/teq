// The collections' ordering and numeric evidence is an implicit parameter, as scala-library
// declares it: an explicit argument goes to it without `using`.
import scala.annotation.nowarn
object Main:
  @nowarn def main(args: Array[String]): Unit =
    val xs = List(3, 1, 2)
    println(xs.sorted(Ordering.Int.reverse))
    println(xs.max(Ordering.Int.reverse) + " " + xs.min(Ordering.Int.reverse))
    println(xs.sortBy(x => -x)(Ordering.Int.reverse))
    println(xs.sum(Numeric.IntIsIntegral) + " " + xs.maxBy(x => -x)(Ordering.Int))
    println(Vector(2, 9, 4).sorted(Ordering.Int))
    println(Array(5, 4).sorted(Ordering.Int).toList)
    println(xs.sorted(using Ordering.Int))
    println(xs.sorted)

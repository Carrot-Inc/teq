// jars: scala-library
// A tuple's elements implement scala-library's `ProductN._1` and the rest, which its `productElement`
// and a value typed `ProductN` read as methods: the tuple's own reads, its pattern, its mirror and
// its equality agree with those.
import scala.deriving.Mirror
final case class Pair(n: Int, s: String)
final case class Cell(_1: Int, _2: String) extends Product2[Int, String]

def firsts[A](ps: List[Product2[A, Any]]): List[A] = ps.map(_._1)

object Main:
  def main(args: Array[String]): Unit =
    val t = (1, "a", true)
    println(t.productIterator.toList)
    println(t.productElement(2))
    val q: Product = (7, "q")
    println(q.productElement(0))
    println(q.productIterator.mkString("<", "|", ">"))
    val p2: Product2[Int, String] = (2, "b")
    println(s"${p2._1}${p2._2}")
    println(firsts(List((1, "x"), Cell(2, "y"), (3, 'z'))))
    println(summon[Mirror.ProductOf[Pair]].fromProduct((3, "c")))
    println(Tuple1(4).productElement(0))
    val u = (5, "e")
    u match
      case (n, s) => println(s"$n $s")
    println(u.swap)
    println(u._1 + u._2.length)
    println(u == (5, "e"))
    println(u.toString)
    val m = Map(1 -> "one", 2 -> "two")
    println(m.map(e => e._2 -> e._1).toList.sortBy(_._2))
    for (k, v) <- m.toList do println(s"$k=$v")

// jars: scala-library
// std: scala-library
// scala-library's `collection.Seq` as a sequence pattern (`case collection.Seq(a, b, ..)` of
// http4s' `Rfc3986`), and the other sequence classes deriving from it, in link mode.
object Main:
  def describe(xs: collection.Seq[Int]): String = xs match
    case collection.Seq() => "empty"
    case collection.Seq(a) => s"one $a"
    case collection.Seq(a, b, rest*) => s"$a $b +${rest.size}"

  def main(args: Array[String]): Unit =
    println(describe(Nil))
    println(describe(Vector(7)))
    println(describe(List(1, 2, 3, 4)))
    println(describe(collection.mutable.ArrayBuffer(5, 6)))
    collection.IndexedSeq(1, 2) match
      case collection.IndexedSeq(x, y) => println(x + y)
      case _ => println("other")

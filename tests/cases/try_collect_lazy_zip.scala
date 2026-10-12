// scala-library's Try.collect and Iterable.lazyZip, which the lean std gives as well.
import scala.util.Try

@main def main(): Unit =
  println(Try(4).collect { case n if n % 2 == 0 => n * 10 })
  val odd = Try(3).collect { case n if n % 2 == 0 => n * 10 }
  println(odd.isFailure)
  println(odd.failed.get.getMessage)
  val xs = List(1, 2, 3)
  val ys = Vector("a", "b", "c", "d")
  println(xs.lazyZip(ys).map((n, s) => s * n))
  println(xs.lazyZip(ys).filter((n, _) => n != 2))
  println(xs.lazyZip(ys).exists((n, s) => n == 3 && s == "c"))
  println(xs.lazyZip(ys).forall((n, _) => n > 0))
  xs.lazyZip(ys).foreach((n, s) => print(s"$s$n "))
  println()

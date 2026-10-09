// Adapted from scala3 tests/run/enum-List1.scala (Apache-2.0, see tests/scala3/README.md).
enum List[T] {
  case Cons(x: T, xs: List[T])
  case Nil()
}
object Test {
  import List.*
  val xs = Cons(1, Cons(2, Cons(3, Nil())))
  def main(args: Array[String]) = println(xs)
}

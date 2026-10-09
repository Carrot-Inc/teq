// A type constructor given for a higher-kinded parameter has to conform to the parameter's
// bound, applied: `Box[X] <: Iterable[X]` does not hold, `List[X] <: Iterable[X]` does.
// expect: type argument Box does not conform to upper bound
// expect: type argument Pair does not conform to upper bound
object Enc:
  def iterable[A, T[X] <: Iterable[X]](a: A): String = "i"
  def keyed[K, V, T[X, Y] <: Iterable[(X, Y)]](k: K, v: V): String = "k"
final case class Box[A](a: A)
final case class Pair[A, B](a: A, b: B)
object Main:
  def main(args: Array[String]): Unit =
    println(Enc.iterable[String, Box]("x"))
    println(Enc.iterable[String, List]("x"))
    println(Enc.keyed[Int, String, Pair](1, "x"))
    println(Enc.keyed[Int, String, Map](1, "x"))

// A value of an applied higher-kinded type parameter compares with null when the parameter's
// bound is a reference type, as boopickle's `MapPickler` checks `map == null`.
object Main:
  def isMissing[T, S, V[_, _] <: collection.Map[?, ?]](map: V[T, S]): Boolean = map == null
  def orEmpty[A, F[_] <: Iterable[?]](xs: F[A], empty: F[A]): F[A] = if xs != null then xs else empty

  def main(args: Array[String]): Unit =
    println(isMissing[Int, String, Map](Map(1 -> "a")))
    println(isMissing[Int, String, Map](null))
    println(orEmpty[Int, List](null, List(1, 2)))

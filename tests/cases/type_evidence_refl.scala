// The evidence the compiler supplies for `A =:= B` and `A <:< B` is scala-library's `<:<.refl[A]`, which
// dotty's search finds (an `implicit def` of type `A =:= A`) and a pickle carries: the identity function, which
// converts a value at its type.
def widen[A, B](a: A)(using ev: A <:< B): B = ev(a)
def same[A, B](a: A)(using ev: A =:= B): B = ev(a)

@main def main(): Unit =
  println(widen[Int, AnyVal](1))
  println(same[String, String]("s"))
  println(summon[Int =:= Int](3) + 1)
  val l: List[Any] = summon[List[Int] <:< List[Any]](List(1, 2))
  println(l)

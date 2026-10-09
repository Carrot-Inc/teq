// The f interpolator's type parameter, which scala-library declares `[A >: Any]` and a library
// body passes explicitly.
@main def main(): Unit =
  val n = 42
  println(f"n=$n%04d")
  println(StringContext("took ", "%d.", "%09d s").f[Any](3L, 5L))

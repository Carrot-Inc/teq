// An `inline val` of a stored body is checked where the body expands, as scalac 3.8.4's
// `InlineVals` leaves an inline method's body alone: one in a branch the expansion discards is
// never checked (`choose(true)`), one of a body never expanded neither (`unused`), and one whose
// declared type is no literal type stands for its constant (`declared`).
def runtime(): Int = 2
inline def choose(inline b: Boolean): Int =
  inline if b then 1
  else
    inline val x = runtime()
    x
inline def unused: List[Int] =
  inline val xs = List(1)
  xs
inline def declared: Int =
  inline val x: Int = 3
  x
@main def main(): Unit =
  println(choose(true))
  println(declared)
